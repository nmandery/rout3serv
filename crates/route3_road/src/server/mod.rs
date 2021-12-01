use std::convert::TryFrom;
use std::sync::Arc;

use eyre::Result;
use h3ron::collections::H3CellSet;
use h3ron::H3Cell;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tower_http::trace::TraceLayer;

use crate::config::ServerConfig;
use crate::server::api::generated::route3_road_server::{Route3Road, Route3RoadServer};
use crate::server::api::generated::{
    DifferentialShortestPathRequest, DifferentialShortestPathRoutes,
    DifferentialShortestPathRoutesRequest, Empty, H3ShortestPathRequest, IdRef,
    ListDatasetsResponse, ListGraphsResponse, VersionResponse,
};
use crate::server::storage::S3Storage;
use crate::server::util::{spawn_blocking_status, stream_dataframe, ArrowRecordBatchStream};
use crate::weight::RoadWeight;

mod api;
mod differential_shortest_path;
mod shortest_path;
mod storage;
mod util;
mod vector;

struct ServerImpl {
    storage: Arc<S3Storage<RoadWeight>>,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let storage = Arc::new(S3Storage::<RoadWeight>::from_config(Arc::new(config))?);
        Ok(Self { storage })
    }
}

#[tonic::async_trait]
impl Route3Road for ServerImpl {
    async fn version(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<Response<VersionResponse>, Status> {
        Ok(Response::new(VersionResponse {
            version: crate::build_info::version().to_string(),
            git_commit_sha: crate::build_info::git_comit_sha().to_string(),
            build_timestamp: crate::build_info::build_timestamp().to_string(),
        }))
    }
    async fn list_graphs(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ListGraphsResponse>, Status> {
        let mut resp = ListGraphsResponse { graphs: vec![] };

        for gck in self.storage.load_graph_cache_keys().await? {
            /*
            let graph = self.storage.graph_store.load_cached(&gck).await;
            let mut gi: GraphInfo = gck.into();
            if let Some(g) = graph {
                gi.is_cached = true;
                let stats = g.get_stats();
                gi.num_nodes = stats.num_nodes as u64;
                gi.num_edges = stats.num_edges as u64;
            } else {
                gi.is_cached = false;
            }

            resp.graphs.push(gi);
             */
            resp.graphs.push(gck.into());
        }
        Ok(Response::new(resp))
    }

    async fn list_datasets(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ListDatasetsResponse>, Status> {
        let response = ListDatasetsResponse {
            dataset_name: self.storage.list_datasets(),
        };
        Ok(Response::new(response))
    }

    type H3ShortestPathStream = ArrowRecordBatchStream;

    async fn h3_shortest_path(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> std::result::Result<Response<Self::H3ShortestPathStream>, Status> {
        let parameters =
            shortest_path::create_parameters(request.into_inner(), self.storage.clone()).await?;
        shortest_path::h3_shortest_path(parameters).await
    }

    type DifferentialShortestPathStream = ArrowRecordBatchStream;

    async fn differential_shortest_path(
        &self,
        request: Request<DifferentialShortestPathRequest>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let dsp_request = request.into_inner();
        let input =
            differential_shortest_path::collect_input(dsp_request, self.storage.clone()).await?;

        let do_store_output = input.store_output;
        let output = spawn_blocking_status(move || differential_shortest_path::calculate(input))
            .await?
            .map_err(|e| {
                log::error!("calculating routes failed: {:?}", e);
                Status::internal("calculating routes failed")
            })?;

        let response_fut = stream_dataframe(
            output.object_id.clone(),
            differential_shortest_path::disturbance_statistics(&output)?,
        );

        let response = if do_store_output {
            let (_, response) = tokio::try_join!(
                self.storage.store_output(&output), // save the output for later
                response_fut
            )?;
            response
        } else {
            response_fut.await?
        };
        Ok(response)
    }

    type GetDifferentialShortestPathStream = ArrowRecordBatchStream;

    async fn get_differential_shortest_path(
        &self,
        request: Request<IdRef>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let inner = request.into_inner();
        let output: differential_shortest_path::DspOutput<RoadWeight> = self
            .storage
            .retrieve_output(inner.object_id.as_str())
            .await?;

        stream_dataframe(
            output.object_id.clone(),
            differential_shortest_path::disturbance_statistics(&output)?,
        )
        .await
    }

    type GetDifferentialShortestPathRoutesStream =
        ReceiverStream<Result<DifferentialShortestPathRoutes, Status>>;

    async fn get_differential_shortest_path_routes(
        &self,
        request: Request<DifferentialShortestPathRoutesRequest>,
    ) -> Result<Response<Self::GetDifferentialShortestPathRoutesStream>, Status> {
        let (tx, rx) = mpsc::channel(20);
        let inner = request.into_inner();
        let output: differential_shortest_path::DspOutput<RoadWeight> = self
            .storage
            .retrieve_output(inner.object_id.as_str())
            .await?;

        tokio::spawn(async move {
            let cell_lookup: H3CellSet = inner
                .cells
                .iter()
                .filter_map(|h3index| match H3Cell::try_from(*h3index) {
                    Ok(cell) => Some(cell),
                    Err(_) => {
                        log::warn!("received invalid h3index: {}", h3index);
                        None
                    }
                })
                .collect();

            for (origin_cell, diff) in output.differential_shortest_paths.iter() {
                if cell_lookup.contains(origin_cell) {
                    if let Err(e) = tx
                        .send(differential_shortest_path::build_routes_response(diff))
                        .await
                    {
                        log::warn!("streaming of routes aborted. reason: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
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
        .layer(TraceLayer::new_for_grpc())
        .add_service(Route3RoadServer::new(server_impl).send_gzip().accept_gzip())
        .serve(addr)
        .await?;
    Ok(())
}
