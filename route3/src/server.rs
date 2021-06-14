use eyre::Result;
use serde::Deserialize;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use api::route3_server::{Route3, Route3Server};
use api::{VersionRequest, VersionResponse};

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
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
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
    let server_impl = ServerImpl::default();

    println!("Route3 listening on {}", addr);

    Server::builder()
        .add_service(Route3Server::new(server_impl))
        .serve(addr)
        .await?;

    Ok(())
}
