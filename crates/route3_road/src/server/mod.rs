use std::convert::TryFrom;
use std::sync::Arc;

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
    DifferentialShortestPathRoutesRequest, Empty, H3ShortestPathRequest, H3WithinThresholdRequest,
    IdRef, ListDatasetsResponse, ListGraphsResponse, RouteH3Indexes, RouteWkb, VersionResponse,
};
use crate::server::api::RouteH3IndexesKind;
use crate::server::storage::S3Storage;
use crate::server::util::{spawn_blocking_status, stream_dataframe, ArrowIpcChunkStream};
use crate::weight::RoadWeight;

mod api;
mod differential_shortest_path;
mod names;
mod shortest_path;
mod storage;
mod util;
mod vector;
mod within_threshold;

struct ServerImpl {
    storage: Arc<S3Storage<RoadWeight>>,
    config: Arc<ServerConfig>,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> eyre::Result<Self> {
        let config = Arc::new(config);
        let storage = Arc::new(S3Storage::<RoadWeight>::from_config(config.clone())?);
        Ok(Self { storage, config })
    }
}

#[tonic::async_trait]
impl Route3Road for ServerImpl {
    async fn version(&self, _request: Request<Empty>) -> Result<Response<VersionResponse>, Status> {
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

    type H3ShortestPathStream = ArrowIpcChunkStream;

    async fn h3_shortest_path(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathStream>, Status> {
        let parameters = shortest_path::create_parameters(
            request.into_inner(),
            self.storage.clone(),
            self.config.clone(),
        )
        .await?;
        shortest_path::h3_shortest_path(parameters).await
    }

    type H3ShortestPathRoutesStream = ReceiverStream<Result<RouteWkb, Status>>;

    async fn h3_shortest_path_routes(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathRoutesStream>, Status> {
        let req = request.into_inner();
        let smoothen_geometries = req.smoothen_geometries;
        let parameters =
            shortest_path::create_parameters(req, self.storage.clone(), self.config.clone())
                .await?;
        shortest_path::h3_shortest_path_routes(parameters, move |p| {
            RouteWkb::from_path(&p, smoothen_geometries)
        })
        .await
    }

    type H3ShortestPathCellsStream = ReceiverStream<Result<RouteH3Indexes, Status>>;

    async fn h3_shortest_path_cells(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathCellsStream>, Status> {
        let parameters = shortest_path::create_parameters(
            request.into_inner(),
            self.storage.clone(),
            self.config.clone(),
        )
        .await?;
        shortest_path::h3_shortest_path_routes(parameters, move |p| {
            RouteH3Indexes::from_path(&p, RouteH3IndexesKind::Cells)
        })
        .await
    }

    type H3ShortestPathEdgesStream = ReceiverStream<Result<RouteH3Indexes, Status>>;

    async fn h3_shortest_path_edges(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathEdgesStream>, Status> {
        let parameters = shortest_path::create_parameters(
            request.into_inner(),
            self.storage.clone(),
            self.config.clone(),
        )
        .await?;
        shortest_path::h3_shortest_path_routes(parameters, move |p| {
            RouteH3Indexes::from_path(&p, RouteH3IndexesKind::Edges)
        })
        .await
    }

    type DifferentialShortestPathStream = ArrowIpcChunkStream;

    async fn differential_shortest_path(
        &self,
        request: Request<DifferentialShortestPathRequest>,
    ) -> Result<Response<ArrowIpcChunkStream>, Status> {
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

    type GetDifferentialShortestPathStream = ArrowIpcChunkStream;

    async fn get_differential_shortest_path(
        &self,
        request: Request<IdRef>,
    ) -> Result<Response<ArrowIpcChunkStream>, Status> {
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
                .filter_map(|h3index| {
                    if let Ok(cell) = H3Cell::try_from(*h3index) {
                        Some(cell)
                    } else {
                        log::warn!("received invalid h3index: {}", h3index);
                        None
                    }
                })
                .collect();

            for (origin_cell, diff) in output.differential_shortest_paths.iter() {
                if cell_lookup.contains(origin_cell) {
                    if let Err(e) = tx
                        .send(differential_shortest_path::build_routes_response(
                            diff,
                            inner.smoothen_geometries,
                        ))
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

    type H3CellsWithinThresholdStream = ArrowIpcChunkStream;

    async fn h3_cells_within_threshold(
        &self,
        request: Request<H3WithinThresholdRequest>,
    ) -> Result<Response<Self::H3CellsWithinThresholdStream>, Status> {
        let parameters = within_threshold::create_parameters(
            request.into_inner(),
            self.storage.clone(),
            self.config.clone(),
        )
        .await?;
        within_threshold::within_threshold(parameters).await
    }
}

pub fn launch_server(server_config: ServerConfig) -> eyre::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server(server_config))?;
    Ok(())
}

async fn run_server(server_config: ServerConfig) -> eyre::Result<()> {
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
