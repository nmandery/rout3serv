use std::convert::TryFrom;
use std::sync::Arc;

use eyre::Result;
use h3ron::collections::{H3CellMap, H3CellSet};
use h3ron::H3Cell;
use h3ron::HasH3Resolution;
use h3ron_graph::graph::GetStats;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use crate::config::ServerConfig;
use crate::io::graph_store::GraphCacheKey;
use crate::io::s3::FoundOption;
use crate::server::api::generated::route3_road_server::{Route3Road, Route3RoadServer};
use crate::server::api::generated::{
    DifferentialShortestPathRequest, DifferentialShortestPathRoutes,
    DifferentialShortestPathRoutesRequest, Empty, GraphInfo, GraphInfoResponse, IdRef,
    VersionResponse,
};
use crate::server::storage::S3Storage;
use crate::server::util::{
    respond_recordbatches_stream, spawn_blocking_status, ArrowRecordBatchStream,
};
use crate::weight::Weight;

mod api;
mod differential_shortest_path;
mod storage;
mod util;
mod vector;

struct ServerImpl {
    storage: S3Storage<Weight>,
}

impl ServerImpl {
    pub async fn create(config: ServerConfig) -> Result<Self> {
        let storage = S3Storage::<Weight>::from_config(Arc::new(config))?;
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
    async fn graph_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<GraphInfoResponse>, Status> {
        let mut resp = GraphInfoResponse { graphs: vec![] };

        for gck in self.storage.load_graph_cache_keys().await? {
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
        }
        Ok(Response::new(resp))
    }

    type DifferentialShortestPathStream = ArrowRecordBatchStream;

    async fn differential_shortest_path(
        &self,
        request: Request<DifferentialShortestPathRequest>,
    ) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
        let dsp_request = request.into_inner();
        let (graph, graph_cache_key) = self
            .storage
            .load_graph_from_option(&dsp_request.graph_handle)
            .await?;
        let input = dsp_request.get_input(graph.h3_resolution())?;

        // obtain the downsampled graph if requested
        let downsampled_graph = if input.downsampled_prerouting {
            // attempt to find a suitable graph at a lower resolution
            let mut found_gck: Option<GraphCacheKey> = None;
            for gck in self.storage.load_graph_cache_keys().await?.drain(..) {
                if gck.name == graph_cache_key.name
                    && gck.h3_resolution < graph_cache_key.h3_resolution
                {
                    if let Some(f_gck) = found_gck.as_ref() {
                        // use the next lower resolution
                        if f_gck.h3_resolution < gck.h3_resolution {
                            found_gck = Some(gck);
                        }
                    } else {
                        found_gck = Some(gck);
                    }
                }
            }

            if let Some(dsg_gck) = found_gck {
                Some(self.storage.load_graph(&dsg_gck).await?)
            } else {
                return Err(Status::invalid_argument(
                    "no suitable graph at a lower resolution found",
                ));
            }
        } else {
            None
        };

        let population: H3CellMap<f32> = self
            .storage
            .load_population(graph.h3_resolution(), &input.within_buffer)
            .await?;

        let do_store_output = input.store_output;
        let output = spawn_blocking_status(move || {
            differential_shortest_path::calculate(graph, input, population, downsampled_graph)
        })
        .await?
        .map_err(|e| {
            log::error!("calculating routes failed: {:?}", e);
            Status::internal("calculating routes failed")
        })?;

        let response_fut = respond_recordbatches_stream(
            output.object_id.clone(),
            differential_shortest_path::disturbance_statistics_status(&output)?,
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
        if let FoundOption::Found(output) = self
            .storage
            .retrieve_output::<_, differential_shortest_path::DspOutput>(inner.object_id.as_str())
            .await?
        {
            respond_recordbatches_stream(
                output.object_id.clone(),
                differential_shortest_path::disturbance_statistics_status(&output)?,
            )
            .await
        } else {
            Err(Status::not_found("not found"))
        }
    }

    type GetDifferentialShortestPathRoutesStream =
        ReceiverStream<Result<DifferentialShortestPathRoutes, Status>>;

    async fn get_differential_shortest_path_routes(
        &self,
        request: Request<DifferentialShortestPathRoutesRequest>,
    ) -> Result<Response<Self::GetDifferentialShortestPathRoutesStream>, Status> {
        let (tx, rx) = mpsc::channel(20);
        let inner = request.into_inner();
        let output = if let FoundOption::Found(output) = self
            .storage
            .retrieve_output::<_, differential_shortest_path::DspOutput>(inner.object_id.as_str())
            .await?
        {
            output
        } else {
            return Err(Status::not_found("not found"));
        };

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
                    tx.send(differential_shortest_path::build_routes_response(diff))
                        .await
                        .unwrap();
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
        .add_service(Route3RoadServer::new(server_impl))
        .serve(addr)
        .await?;
    Ok(())
}
