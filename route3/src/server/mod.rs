use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::Write;
use std::iter::FromIterator;
use std::sync::Arc;

use arrow::array::{Float32Array, UInt64Array};
use eyre::{Report, Result};
use geojson::FeatureCollection;
use serde::Deserialize;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use api::route3_server::{Route3, Route3Server};
use api::{
    DisturbanceOfPopulationMovementInput, DisturbanceOfPopulationMovementRequest,
    DisturbanceOfPopulationMovementResponse, VersionRequest, VersionResponse,
};
use route3_core::geo_types::GeometryCollection;
use route3_core::graph::H3Graph;
use route3_core::h3ron::H3Cell;
use route3_core::io::load_graph_from_byte_slice;
use route3_core::routing::RoutingContext;
use route3_core::{H3CellMap, H3CellSet, WithH3Resolution};

use crate::constants::Weight;
use crate::io::s3::{S3Client, S3Config, S3H3Dataset, S3RecordBatchLoader};
use crate::io::{recordbatch_array, FoundOption};
use crate::server::algo::{disturbance_of_population_movement, StorableOutput};
use crate::server::util::spawn_blocking_status;

mod algo;
mod api;
mod util;

struct ServerImpl {
    config: ServerConfig,
    s3_client: Arc<S3Client>,
    routing_context: Arc<RoutingContext<Weight>>,
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

        Ok(Self {
            config,
            s3_client,
            routing_context: Arc::new(graph.try_into()?),
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
                self.routing_context.h3_resolution(),
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

    async fn store_output(&self, output: &StorableOutput) -> std::result::Result<(), Status> {
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

    async fn retrieve_output<I: AsRef<str>>(
        &self,
        id: I,
    ) -> std::result::Result<FoundOption<StorableOutput>, Status> {
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
                let storable_output: StorableOutput =
                    bincode::deserialize(&bytes).map_err(|e| {
                        log::error!("deserializing output with key = {} failed: {:?}", key, e);
                        Status::internal(format!("deserializing output with key = {} failed", key))
                    })?;
                FoundOption::Found(storable_output)
            }
            FoundOption::NotFound => FoundOption::NotFound,
        };
        Ok(found_option)
    }
}

#[tonic::async_trait]
impl Route3 for ServerImpl {
    async fn version(
        &self,
        _request: Request<VersionRequest>,
    ) -> std::result::Result<Response<VersionResponse>, Status> {
        Ok(Response::new(VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn analyze_disturbance_of_population_movement(
        &self,
        request: Request<DisturbanceOfPopulationMovementRequest>,
    ) -> std::result::Result<Response<DisturbanceOfPopulationMovementResponse>, Status> {
        let input = request
            .into_inner()
            .get_input(self.routing_context.h3_resolution())?;

        let population = self.load_population(&input.within_buffer).await?;
        let routing_context = self.routing_context.clone();
        let output = spawn_blocking_status(move || {
            disturbance_of_population_movement(routing_context, input, population)
        })
        .await?
        .map_err(|e| {
            log::error!("calculating routes failed: {:?}", e);
            Status::internal("calculating routes failed")
        })?;

        /*
        let gc = GeometryCollection::from_iter(routes.iter().map(|route| route.to_linestring()));
        let fc = FeatureCollection::from(&gc);
        let mut f = File::create("/tmp/gj.json").unwrap();
        f.write_all(fc.to_string().as_bytes()).unwrap();

         */
        let response = DisturbanceOfPopulationMovementResponse {
            id: output.id.clone(),
            population_within_disturbance: output.population_within_disturbance,
        };

        // save the output for later
        self.store_output(&output.into()).await?;

        Ok(Response::new(response))
    }
}

#[derive(Deserialize)]
pub struct GraphConfig {
    key: String,
    bucket: String,
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
        .add_service(Route3Server::new(server_impl))
        .serve(addr)
        .await?;

    Ok(())
}
