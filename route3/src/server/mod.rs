use std::collections::{HashMap, HashSet};
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
use api::{AnalyzeDisturbanceRequest, AnalyzeDisturbanceResponse, VersionRequest, VersionResponse};
use route3_core::geo_types::GeometryCollection;
use route3_core::graph::H3Graph;
use route3_core::h3ron::H3Cell;
use route3_core::io::load_graph_from_byte_slice;
use route3_core::routing::{ManyToManyOptions, RoutingContext};
use route3_core::WithH3Resolution;

use crate::constants::WeightType;
use crate::io::recordbatch_array;
use crate::io::s3::{ObjectBytes, S3Client, S3Config, S3H3Dataset, S3RecordBatchLoader};
use crate::server::util::spawn_blocking_status;

mod api;
mod util;

struct ServerImpl {
    config: ServerConfig,
    s3_client: Arc<S3Client>,
    routing_context: Arc<RoutingContext<WeightType>>,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let s3_client = Arc::new(S3Client::from_config(&config.s3)?);

        let graph: H3Graph<WeightType> = match s3_client
            .get_object_bytes(config.graph.bucket.clone(), config.graph.key.clone())
            .await?
        {
            ObjectBytes::Found(graph_bytes) => load_graph_from_byte_slice(&graph_bytes)?,
            ObjectBytes::NotFound => return Err(Report::msg("could not find graph")),
        };

        if config.population_dataset.file_h3_resolution > graph.h3_resolution {
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
    ) -> std::result::Result<HashMap<H3Cell, f32>, Status> {
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
        let mut population_cells = HashMap::new();
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

            let cells_to_use: HashSet<_> = cells.iter().collect();
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

    async fn analyze_disturbance(
        &self,
        request: Request<AnalyzeDisturbanceRequest>,
    ) -> std::result::Result<Response<AnalyzeDisturbanceResponse>, Status> {
        let input = request
            .into_inner()
            .get_input(self.routing_context.h3_resolution())?;

        let population = self.load_population(&input.within_buffer).await?;
        let population_within_disturbance = input
            .disturbance
            .iter()
            .filter_map(|cell| population.get(cell))
            .sum::<f32>() as f64;

        let routing_start_cells: Vec<H3Cell> = input
            .within_buffer
            .iter()
            .filter(|cell| !input.disturbance.contains(cell))
            .cloned()
            .collect();

        let routing_context = self.routing_context.clone();
        let routes = spawn_blocking_status(move || {
            let options = ManyToManyOptions {
                num_destinations_to_reach: input.num_destinations_to_reach,
            };
            routing_context.route_many_to_many(&routing_start_cells, &input.destinations, &options)
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

        Ok(Response::new(AnalyzeDisturbanceResponse {
            population_within_disturbance,
        }))
    }
}

#[derive(Deserialize)]
pub struct GraphConfig {
    key: String,
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
