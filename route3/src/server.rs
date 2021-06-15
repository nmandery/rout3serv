use std::io::Cursor;

use eyre::{Report, Result};
use h3ron::ToH3Indexes;
use serde::Deserialize;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use api::route3_server::{Route3, Route3Server};
use api::{AnalyzeDisturbanceRequest, AnalyzeDisturbanceResponse, VersionRequest, VersionResponse};

use crate::graph::Graph;
use crate::io::load_graph;
use crate::io::s3::{ObjectBytes, S3Client, S3Config};

mod api {
    use tonic::include_proto;

    include_proto!("grpc.route3");
}

struct ServerImpl {
    config: ServerConfig,
    s3_client: S3Client,
    graph: Graph,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let s3_client = S3Client::from_config(&config.s3)?;

        let graph = match s3_client
            .get_object_bytes(&config.graph.bucket, &config.graph.key)
            .await?
        {
            ObjectBytes::Found(graph_bytes) => {
                let mut cursor = Cursor::new(&graph_bytes);
                load_graph(cursor)?
            }
            ObjectBytes::NotFound => return Err(Report::msg("could not find graph")),
        };

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
            .map_err(|e| Status::invalid_argument("could not parse WKB"))?
            .to_h3_indexes(self.graph.h3_resolution)
            .map_err(|e| Status::internal("could not convert to h3"))?;

        // remove duplicates in case of multi* geometries
        cells.sort_unstable();
        cells.dedup();

        dbg!(cells.len());
        Ok(Response::new(AnalyzeDisturbanceResponse {}))
    }
}

#[derive(Deserialize)]
pub struct GraphConfig {
    key: String,
    bucket: String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub s3: S3Config,
    pub graph: GraphConfig,
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
