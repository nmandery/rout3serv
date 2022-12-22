use std::convert::TryFrom;
use std::ops::Add;
use std::sync::Arc;

use h3ron::collections::H3CellSet;
use h3ron::iter::change_resolution;
use h3ron::H3Cell;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use h3ron_polars::frame::H3DataFrame;
use num_traits::Zero;
use object_store::path::Path;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::block_in_place;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codec::CompressionEncoding;
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status};
use tower_http::trace::TraceLayer;
use tracing::{info, log::Level, warn};

use crate::config::ServerConfig;
use crate::grpc::api::generated::rout3_serv_server::{Rout3Serv, Rout3ServServer};
use crate::grpc::api::generated::{
    CellSelection, DifferentialShortestPathRequest, DifferentialShortestPathRoutes,
    DifferentialShortestPathRoutesRequest, Empty, GraphHandle, H3ShortestPathRequest,
    H3WithinThresholdRequest, IdRef, ListDatasetsResponse, ListGraphsResponse, RouteH3Indexes,
    RouteWkb, VersionResponse,
};
use crate::grpc::api::RouteH3IndexesKind;
use crate::grpc::error::ToStatusResult;
use crate::grpc::error::{logged_status, StatusCodeAndMessage};
use crate::grpc::util::{spawn_blocking_status, stream_dataframe, ArrowIpcChunkStream};
use crate::io::dataframe::DataframeDataset;
use crate::io::{GraphKey, Storage};
use crate::weight::{RoadWeight, Weight};

mod api;
mod differential_shortest_path;
mod error;
mod names;
mod shortest_path;
mod util;
mod vector;
mod within_threshold;

pub trait ServerWeight:
    Send
    + Sync
    + Serialize
    + DeserializeOwned
    + Weight
    + PartialOrd
    + PartialEq
    + Add
    + Copy
    + Ord
    + Zero
{
}

pub struct LoadedCellSelection {
    pub cells: Vec<H3Cell>,
    pub dataframe: Option<H3DataFrame<H3Cell>>,
}

pub(crate) struct ServerImpl<W: ServerWeight> {
    storage: Arc<Storage<W>>,
    config: Arc<ServerConfig>,
}

impl<W: ServerWeight> ServerImpl<W> {
    pub async fn create(config: ServerConfig) -> anyhow::Result<Self> {
        let config = Arc::new(config);
        let storage = Arc::new(Storage::from_config(&config)?);
        Ok(Self { storage, config })
    }

    fn build_output_key(&self, output_id: &str) -> Path {
        format!("{}{}", self.config.outputs.prefix, output_id).into()
    }

    async fn retrieve_graph_by_handle(
        &self,
        graph_handle: &Option<GraphHandle>,
    ) -> Result<(Arc<PreparedH3EdgeGraph<W>>, GraphKey), Status> {
        let gk: GraphKey = graph_handle.try_into()?;
        self.storage
            .retrieve_graph(gk.clone())
            .await
            .to_status_result()
            .map(|g| (g, gk))
    }

    fn dataset_by_name(&self, dataset_name: &str) -> Result<&DataframeDataset, Status> {
        self.config.datasets.get(dataset_name).ok_or_else(|| {
            logged_status(
                format!("not such dataset: {}", dataset_name),
                Code::NotFound,
                Level::Debug,
            )
        })
    }

    /// fetch all contents required for the `cell_selection`.
    ///
    /// Input cells will get:
    /// * transformed to `h3_resolution`
    /// * filtered by the dataset given using the `dataset_name` in the `CellSelection`
    /// * invalid cells will get removed
    ///
    /// In case the `dataset_name` is set, the `DataFrame` for this dataset will
    /// be returned as well.
    pub async fn load_cell_selection(
        &self,
        cell_selection: &Option<CellSelection>,
        h3_resolution: u8,
        selection_name: &str,
    ) -> Result<LoadedCellSelection, Status> {
        let Some(cell_selection) = cell_selection else { return Err(logged_status(format!("empty cell selection '{}' given", selection_name), Code::InvalidArgument, Level::Info)) };

        // build a complete list of the requested h3cells transformed to the
        // correct resolution
        let mut cells = block_in_place(|| {
            change_resolution(
                cell_selection.cells.iter().filter_map(|v| {
                    if let Ok(cell) = H3Cell::try_from(*v) {
                        Some(cell)
                    } else {
                        warn!("invalid h3 index {} ignored", v);
                        None
                    }
                }),
                h3_resolution,
            )
            .collect::<Result<Vec<_>, _>>()
            .to_status_result_with_message(Code::Internal, || {
                "transforming input cell selection resolution failed".to_string()
            })
            .map(|mut cells| {
                cells.sort_unstable();
                cells.dedup();
                cells
            })
        })?;

        if cells.is_empty() || cell_selection.dataset_name.is_empty() {
            Ok(LoadedCellSelection {
                cells,
                dataframe: None,
            })
        } else {
            match self
                .storage
                .retrieve_dataframe(
                    self.dataset_by_name(&cell_selection.dataset_name)?,
                    &cells,
                    h3_resolution,
                )
                .await
                .to_status_result()?
            {
                Some(df) => {
                    block_in_place(|| filter_cells_by_dataframe_contents(&df, &mut cells))?;
                    Ok(LoadedCellSelection {
                        cells,
                        dataframe: Some(df),
                    })
                }
                None => Ok(LoadedCellSelection {
                    cells: Default::default(),
                    dataframe: None,
                }),
            }
        }
    }
}

#[tonic::async_trait]
impl<W: ServerWeight + 'static> Rout3Serv for ServerImpl<W> {
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
        let resp = ListGraphsResponse {
            graphs: self
                .storage
                .list_graphs()
                .await
                .to_status_result()?
                .into_iter()
                .map(|graph_key| graph_key.into())
                .collect(),
        };
        Ok(Response::new(resp))
    }

    async fn list_datasets(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<ListDatasetsResponse>, Status> {
        let response = ListDatasetsResponse {
            dataset_name: self.config.datasets.keys().cloned().collect(),
        };
        Ok(Response::new(response))
    }

    type H3ShortestPathStream = ArrowIpcChunkStream;

    async fn h3_shortest_path(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathStream>, Status> {
        shortest_path::h3_shortest_path(
            shortest_path::create_parameters(request.into_inner(), self).await?,
        )
        .await
    }

    type H3ShortestPathRoutesStream = ReceiverStream<Result<RouteWkb, Status>>;

    async fn h3_shortest_path_routes(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathRoutesStream>, Status> {
        let req = request.into_inner();
        let smoothen_geometries = req.smoothen_geometries;
        shortest_path::h3_shortest_path_routes(
            shortest_path::create_parameters(req, self).await?,
            move |p| RouteWkb::from_path(&p, smoothen_geometries),
        )
        .await
    }

    type H3ShortestPathCellsStream = ReceiverStream<Result<RouteH3Indexes, Status>>;

    async fn h3_shortest_path_cells(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathCellsStream>, Status> {
        shortest_path::h3_shortest_path_routes(
            shortest_path::create_parameters(request.into_inner(), self).await?,
            move |p| RouteH3Indexes::from_path(&p, RouteH3IndexesKind::Cells),
        )
        .await
    }

    type H3ShortestPathEdgesStream = ReceiverStream<Result<RouteH3Indexes, Status>>;

    async fn h3_shortest_path_edges(
        &self,
        request: Request<H3ShortestPathRequest>,
    ) -> Result<Response<Self::H3ShortestPathEdgesStream>, Status> {
        shortest_path::h3_shortest_path_routes(
            shortest_path::create_parameters(request.into_inner(), self).await?,
            move |p| RouteH3Indexes::from_path(&p, RouteH3IndexesKind::Edges),
        )
        .await
    }

    type DifferentialShortestPathStream = ArrowIpcChunkStream;

    async fn differential_shortest_path(
        &self,
        request: Request<DifferentialShortestPathRequest>,
    ) -> Result<Response<ArrowIpcChunkStream>, Status> {
        let input = differential_shortest_path::collect_input(request.into_inner(), self).await?;

        let do_store_output = input.store_output;
        let output = spawn_blocking_status(move || differential_shortest_path::calculate(input))
            .await?
            .to_status_result()?;

        let response_fut = stream_dataframe(
            output.object_id.clone(),
            differential_shortest_path::disturbance_statistics(&output)?,
        );

        let response = if do_store_output {
            let path = self.build_output_key(&output.object_id);
            let (_, response) = tokio::try_join!(
                async {
                    self.storage
                        .store(&path, &output)
                        .await
                        .map_err(|e| e.status())
                }, // save the output for later
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
            .retrieve(&self.build_output_key(&inner.object_id))
            .await
            .to_status_result()?;

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
            .retrieve(&self.build_output_key(inner.object_id.as_str()))
            .await
            .to_status_result()?;

        tokio::spawn(async move {
            let cell_lookup: H3CellSet = inner
                .cells
                .iter()
                .filter_map(|h3index| {
                    if let Ok(cell) = H3Cell::try_from(*h3index) {
                        Some(cell)
                    } else {
                        warn!("received invalid h3index: {}", h3index);
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
                        warn!("streaming of routes aborted. reason: {}", e);
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
        within_threshold::within_threshold(
            within_threshold::create_parameters(request.into_inner(), self).await?,
        )
        .await
    }
}

pub fn launch_server<W: ServerWeight + 'static>(server_config: ServerConfig) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server::<W>(server_config))?;
    Ok(())
}

async fn run_server<W: ServerWeight + 'static>(server_config: ServerConfig) -> anyhow::Result<()> {
    let addr = server_config.bind_to.parse()?;
    info!("creating grpc server");
    let server_impl: ServerImpl<W> = ServerImpl::create(server_config).await?;

    info!("{} is listening on {}", env!("CARGO_PKG_NAME"), addr);

    Server::builder()
        .layer(TraceLayer::new_for_grpc())
        .add_service(
            Rout3ServServer::new(server_impl)
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .serve(addr)
        .await?;
    Ok(())
}

fn filter_cells_by_dataframe_contents(
    df: &H3DataFrame<H3Cell>,
    cells: &mut Vec<H3Cell>,
) -> Result<(), Status> {
    if df.dataframe().is_empty() {
        cells.clear();
    } else {
        let df_cells_lookup: H3CellSet = df
            .h3indexchunked()
            .to_status_result()?
            .to_collection()
            .to_status_result()?;
        cells.retain(|cell| df_cells_lookup.contains(cell));
    }
    Ok(())
}
