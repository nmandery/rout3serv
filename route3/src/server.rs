use std::io::Cursor;

use eyre::Result;
use h3ron::ToH3Indexes;
use serde::Deserialize;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use api::route3_server::{Route3, Route3Server};
use api::{AnalyzeDisturbanceRequest, AnalyzeDisturbanceResponse, VersionRequest, VersionResponse};

use crate::io::s3::{S3Client, S3Config};

mod api {
    use tonic::include_proto;

    include_proto!("grpc.route3");
}

#[derive(Default)]
struct ServerImpl {}

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
            .to_h3_indexes(10)
            .map_err(|e| Status::internal("could not convert to h3"))?;

        // remove duplicates in case of multi* geometries
        cells.sort_unstable();
        cells.dedup();

        dbg!(cells.len());
        Ok(Response::new(AnalyzeDisturbanceResponse {}))
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub s3: S3Config,
}

pub fn launch_server(server_config: ServerConfig) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server(&server_config))?;
    Ok(())
}

async fn run_server(server_config: &ServerConfig) -> Result<()> {
    let addr = server_config.bind_to.parse()?;
    let s3_client = S3Client::from_config(&server_config.s3)?;
    let server_impl = ServerImpl::default();

    println!("Route3 listening on {}", addr);

    Server::builder()
        .add_service(Route3Server::new(server_impl))
        .serve(addr)
        .await?;

    Ok(())
}
