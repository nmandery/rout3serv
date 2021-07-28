use std::cmp::min;
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;

use arrow::array::{Float32Array, UInt64Array};
use arrow::record_batch::RecordBatch;
use eyre::{Report, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use route3_core::collections::{H3CellMap, H3CellSet};
use route3_core::graph::{downsample_graph, H3Graph};
use route3_core::h3ron::H3Cell;
use route3_core::io::load_graph_from_byte_slice;
use route3_core::routing::RoutingGraph;
use route3_core::WithH3Resolution;

use crate::io::recordbatch_array;
use crate::io::s3::{FoundOption, S3Client, S3Config, S3H3Dataset, S3RecordBatchLoader};
use crate::server::api::route3_road::route3_road_server::{Route3Road, Route3RoadServer};
use crate::server::api::route3_road::{
    ArrowRecordBatch, DisturbanceOfPopulationMovementRequest,
    DisturbanceOfPopulationMovementRoutes, DisturbanceOfPopulationMovementRoutesRequest, Empty,
    GraphInfoResponse, IdRef, RouteWkb, VersionResponse,
};
use crate::server::util::{recordbatch_to_bytes_status, spawn_blocking_status, StrId};
use crate::types::Weight;

mod api;
mod population_movement;
mod util;

type ArrowRecordBatchStream = ReceiverStream<Result<ArrowRecordBatch, Status>>;

struct ServerImpl {
    config: ServerConfig,
    s3_client: Arc<S3Client>,
    routing_graph: Arc<RoutingGraph<Weight>>,

    /// downsampled routing graph
    ds_routing_graph: Arc<RoutingGraph<Weight>>,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let s3_client = Arc::new(S3Client::from_config(&config.s3)?);

        let graph: H3Graph<Weight> = match s3_client
            .get_object_bytes(config.graph.bucket.clone(), config.graph.key.clone())
            .await?
        {
            FoundOption::Found(graph_bytes) => load_graph_from_byte_slice(&graph_bytes)?,
            FoundOption::NotFound => return Err(Report::msg("could not find graph")),
        };

        if config.population_dataset.file_h3_resolution > graph.h3_resolution() {
            return Err(Report::msg(
                "file_h3_resolution of the population must be lower than the graph h3 resolution",
            ));
        }

        // create the downsampled routing graph. the reduced resolution should be only a bit less than
        // the main graphs resolution to avoid skewing results too much by bridging non-connected nodes.
        let ds_graph_resolution = graph
            .h3_resolution
            .saturating_sub(config.graph.downsample_resolution_difference.unwrap_or(2));
        log::debug!(
            "using h3 resolution = {} for the downsampled graph",
            ds_graph_resolution
        );
        let ds_graph = downsample_graph(&graph, ds_graph_resolution, min)?;

        Ok(Self {
            config,
            s3_client,
            routing_graph: Arc::new(graph.try_into()?),
            ds_routing_graph: Arc::new(ds_graph.try_into()?),
        })
    }

    async fn load_population(
        &self,
        cells: &[H3Cell],
    ) -> std::result::Result<H3CellMap<f32>, Status> {
        let h3index_column_name = self.config.population_dataset.get_h3index_column_name();
        let population_count_column_name = self
            .config
            .population_dataset
            .get_population_count_column_name();

        let loader = S3RecordBatchLoader::new(self.s3_client.clone());
        let population = loader
            .load_h3_dataset(
                self.config.population_dataset.clone(),
                cells,
                self.routing_graph.h3_resolution(),
            )
            .await
            .map_err(|e| {
                log::error!("loading population from s3 failed: {:?}", e);
                Status::internal("population data inaccessible")
            })?;
        let mut population_cells = H3CellMap::new();
        for pop in population.iter() {
            let h3index_array = recordbatch_array::<UInt64Array>(pop, &h3index_column_name)
                .map_err(|report| {
                    log::error!("Can not access population data: {}", report);
                    Status::internal("population h3index is inaccessible")
                })?;
            let pop_array = recordbatch_array::<Float32Array>(pop, &population_count_column_name)
                .map_err(|report| {
                log::error!("Can not access population data: {}", report);
                Status::internal("population h3index is inaccessible")
            })?;

            let cells_to_use: H3CellSet = cells.iter().cloned().collect();
            for (h3index_o, pop_o) in h3index_array.iter().zip(pop_array.iter()) {
                if let (Some(h3index), Some(population_count)) = (h3index_o, pop_o) {
                    if let Ok(cell) = H3Cell::try_from(h3index) {
                        if cells_to_use.contains(&cell) {
                            population_cells.insert(cell, population_count);
                        }
                    } else {
                        log::warn!(
                            "encountered invalid h3 index in population data: {}",
                            h3index
                        );
                    }
                }
            }
        }
        Ok(population_cells)
    }

    fn output_s3_key<I: AsRef<str>>(&self, id: I) -> String {
        format!(
            "{}.bincode",
            self.config
                .output
                .key_prefix
                .as_ref()
                .map(|prefix| format!("{}{}", prefix, id.as_ref()))
                .unwrap_or_else(|| id.as_ref().to_string())
        )
    }

    async fn store_output<O: Serialize + StrId>(
        &self,
        output: &O,
    ) -> std::result::Result<(), Status> {
        let serialized = bincode::serialize(output).map_err(|e| {
            log::error!("serializing output failed: {:?}", e);
            Status::internal("serializing output failed")
        })?;
        self.s3_client
            .put_object_bytes(
                self.config.output.bucket.clone(),
                self.output_s3_key(output.id()),
                serialized,
            )
            .await
            .map_err(|e| {
                log::error!("storing output failed: {:?}", e);
                Status::internal("storing output failed")
            })?;
        Ok(())
    }

    async fn retrieve_output<I: AsRef<str>, O: DeserializeOwned>(
        &self,
        id: I,
    ) -> std::result::Result<FoundOption<O>, Status> {
        let key = self.output_s3_key(id);
        let found_option = match self
            .s3_client
            .get_object_bytes(self.config.output.bucket.clone(), key.clone())
            .await
            .map_err(|e| {
                log::error!("retrieving output with key = {} failed: {:?}", key, e);
                Status::internal(format!("retrieving output with key = {} failed", key))
            })? {
            FoundOption::Found(bytes) => {
                let output: O = bincode::deserialize(&bytes).map_err(|e| {
                    log::error!("deserializing output with key = {} failed: {:?}", key, e);
                    Status::internal(format!("deserializing output with key = {} failed", key))
                })?;
                FoundOption::Found(output)
            }
            FoundOption::NotFound => FoundOption::NotFound,
        };
        Ok(found_option)
    }

    async fn respond_recordbatches_stream(
        &self,
        id: String,
        mut recordbatches: Vec<RecordBatch>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let (tx, rx) = mpsc::channel(5);
        tokio::spawn(async move {
            for recordbatch in recordbatches.drain(..) {
                let serialization_result =
                    recordbatch_to_bytes_status(&recordbatch).map(|rb_bytes| ArrowRecordBatch {
                        object_id: id.clone(),
                        data: rb_bytes,
                    });
                tx.send(serialization_result).await.unwrap();
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tonic::async_trait]
impl Route3Road for ServerImpl {
    type AnalyzeDisturbanceOfPopulationMovementStream =
        ReceiverStream<Result<ArrowRecordBatch, Status>>;

    type GetDisturbanceOfPopulationMovementStream =
        ReceiverStream<Result<ArrowRecordBatch, Status>>;

    type GetDisturbanceOfPopulationMovementRoutesStream =
        ReceiverStream<Result<DisturbanceOfPopulationMovementRoutes, Status>>;

    async fn version(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<VersionResponse>, Status> {
        Ok(Response::new(VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn analyze_disturbance_of_population_movement(
        &self,
        request: Request<DisturbanceOfPopulationMovementRequest>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let input = request
            .into_inner()
            .get_input(self.routing_graph.h3_resolution())?;

        let population = self.load_population(&input.within_buffer).await?;
        let routing_graph = self.routing_graph.clone();
        let ds_routing_graph = if input.downsampled_prerouting {
            Some(self.ds_routing_graph.clone())
        } else {
            None
        };
        let output = spawn_blocking_status(move || {
            population_movement::calculate(routing_graph, input, population, ds_routing_graph)
        })
        .await?
        .map_err(|e| {
            log::error!("calculating routes failed: {:?}", e);
            Status::internal("calculating routes failed")
        })?;

        let (_, response) = tokio::try_join!(
            self.store_output(&output), // save the output for later
            self.respond_recordbatches_stream(
                output.dopm_id.clone(),
                population_movement::disturbance_statistics_status(&output)?,
            )
        )?;
        Ok(response)
    }

    async fn get_disturbance_of_population_movement(
        &self,
        request: Request<IdRef>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let inner = request.into_inner();
        if let FoundOption::Found(output) = self
            .retrieve_output::<_, population_movement::Output>(inner.id.as_str())
            .await?
        {
            self.respond_recordbatches_stream(
                output.dopm_id.clone(),
                population_movement::disturbance_statistics_status(&output)?,
            )
            .await
        } else {
            Err(Status::not_found("not found"))
        }
    }

    async fn get_disturbance_of_population_movement_routes(
        &self,
        request: Request<DisturbanceOfPopulationMovementRoutesRequest>,
    ) -> Result<Response<Self::GetDisturbanceOfPopulationMovementRoutesStream>, Status> {
        let (tx, rx) = mpsc::channel(20);
        let inner = request.into_inner();
        let output = if let FoundOption::Found(output) = self
            .retrieve_output::<_, population_movement::Output>(inner.dopm_id.as_str())
            .await?
        {
            output
        } else {
            return Err(Status::not_found("not found"));
        };

        tokio::spawn(async move {
            for h3index in inner.cells.iter() {
                if let Ok(cell) = H3Cell::try_from(*h3index) {
                    tx.send(build_routes_response(&output, cell)).await.unwrap();
                } else {
                    log::warn!("recieved invalid h3index: {}", h3index);
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn graph_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<GraphInfoResponse>, Status> {
        Ok(Response::new(GraphInfoResponse {
            h3_resolution: self.routing_graph.h3_resolution() as u32,
            num_edges: self.routing_graph.graph.num_edges() as u64,
        }))
    }
}

fn build_routes_response(
    output: &population_movement::Output,
    cell: H3Cell,
) -> Result<DisturbanceOfPopulationMovementRoutes, Status> {
    let mut response = DisturbanceOfPopulationMovementRoutes {
        routes_without_disturbance: vec![],
        routes_with_disturbance: vec![],
    };
    if let Some(routes) = output.routes_without_disturbance.get(&cell) {
        for route in routes {
            response
                .routes_without_disturbance
                .push(RouteWkb::from_route(route)?)
        }
    }
    if let Some(routes) = output.routes_with_disturbance.get(&cell) {
        for route in routes {
            response
                .routes_with_disturbance
                .push(RouteWkb::from_route(route)?)
        }
    }
    Ok(response)
}

#[derive(Deserialize)]
pub struct GraphConfig {
    key: String,
    bucket: String,

    /// number of resolution to downsample the graph to for the
    /// internal downsampled graph
    downsample_resolution_difference: Option<u8>,
}

#[derive(Deserialize)]
pub struct OutputConfig {
    key_prefix: Option<String>,
    bucket: String,
}

#[derive(Deserialize, Clone)]
pub struct PopulationDatasetConfig {
    key_pattern: String,
    bucket: String,
    file_h3_resolution: u8,
    h3index_column_name: Option<String>,
    population_count_column_name: Option<String>,
}

impl PopulationDatasetConfig {
    pub fn get_h3index_column_name(&self) -> String {
        self.h3index_column_name
            .clone()
            .unwrap_or_else(|| "h3index".to_string())
    }

    pub fn get_population_count_column_name(&self) -> String {
        self.population_count_column_name
            .clone()
            .unwrap_or_else(|| "population".to_string())
    }
}

impl S3H3Dataset for PopulationDatasetConfig {
    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn key_pattern(&self) -> String {
        self.key_pattern.clone()
    }

    fn file_h3_resolution(&self) -> u8 {
        self.file_h3_resolution
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub s3: S3Config,
    pub graph: GraphConfig,
    pub population_dataset: PopulationDatasetConfig,
    pub output: OutputConfig,
}

pub fn launch_server(server_config: ServerConfig) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server(server_config))?;
    Ok(())
}

async fn run_server(server_config: ServerConfig) -> Result<()> {
    let addr = server_config.bind_to.parse()?;
    log::info!("creating server");
    let server_impl = ServerImpl::create(server_config).await?;

    log::info!("{} is listening on {}", env!("CARGO_PKG_NAME"), addr);

    Server::builder()
        .add_service(Route3RoadServer::new(server_impl))
        .serve(addr)
        .await?;

    Ok(())
}
