use std::io::Cursor;
use std::sync::Arc;

use eyre::{Report, Result};
use serde::Deserialize;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use api::route3_server::{Route3, Route3Server};
use api::{AnalyzeDisturbanceRequest, AnalyzeDisturbanceResponse, VersionRequest, VersionResponse};
use route3_core::graph::H3Graph;
use route3_core::h3ron::ToH3Indexes;
use route3_core::io::load_graph_from_byte_slice;

use crate::io::s3::{ObjectBytes, S3Client, S3Config, S3H3Dataset, S3RecordBatchLoader};

mod api {
    use tonic::include_proto;

    include_proto!("grpc.route3");
}

struct ServerImpl {
    config: ServerConfig,
    s3_client: Arc<S3Client>,
    graph: H3Graph,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let s3_client = Arc::new(S3Client::from_config(&config.s3)?);

        let graph = match s3_client
            .get_object_bytes(config.graph.bucket.clone(), config.graph.key.clone())
            .await?
        {
            ObjectBytes::Found(graph_bytes) => load_graph_from_byte_slice(&graph_bytes)?,
            ObjectBytes::NotFound => return Err(Report::msg("could not find graph")),
        };

        if config.population_dataset.file_h3_resolution > graph.h3_resolution {
            return Err(Report::msg(
                "file_h3resolution of the population must be lower than the graph h3 resolution",
            ));
        }

        Ok(Self {
            config,
            s3_client,
            graph,
        })
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
        let inner = request.into_inner();
        let mut cursor = Cursor::new(&inner.wkb_geometry);
        let mut cells = wkb::wkb_to_geom(&mut cursor)
            .map_err(|e| {
                log::error!("could not parse wkb: {:?}", e);
                Status::invalid_argument("could not parse WKB")
            })?
            .to_h3_indexes(self.graph.h3_resolution)
            .map_err(|e| {
                log::error!("could not convert to h3: {:?}", e);
                Status::internal("could not convert to h3")
            })?;

        // remove duplicates in case of multi* geometries
        cells.sort_unstable();
        cells.dedup();

        dbg!(cells.len());

        dbg!(self.graph.graph.get_num_nodes());
        let ng = self
            .graph
            .build_graph_without_cells(&cells)
            .map_err(|_| Status::internal("graph"))?;
        dbg!(ng.get_num_nodes());

        let loader = S3RecordBatchLoader::new(self.s3_client.clone());
        let population = loader
            .load_h3_dataset(
                self.config.population_dataset.clone(),
                &cells,
                self.graph.h3_resolution,
            )
            .await
            .map_err(|e| Status::internal("pop"))?;
        for pop in population.iter() {
            dbg!(pop.num_columns(), pop.num_rows());
        }

        Ok(Response::new(AnalyzeDisturbanceResponse {}))
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
