use eyre::Result;
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

pub fn launch_server() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server())?;
    Ok(())
}

async fn run_server() -> Result<()> {
    let addr = "0.0.0.0:7000".parse().unwrap();
    let server_impl = ServerImpl::default();

    println!("Route3 listening on {}", addr);

    Server::builder()
        .add_service(Route3Server::new(server_impl))
        .serve(addr)
        .await?;

    Ok(())
}
