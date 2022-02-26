use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;

use h3ron::{H3Cell, HasH3Resolution, Index};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path::ShortestPathOptions;
use h3ron_graph::algorithm::ShortestPathManyToMany;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uom::si::time::second;

use s3io::dataframe::{inner_join_h3dataframe, H3DataFrame};
use s3io::polars_core::prelude::{DataFrame, NamedFrom, Series};

use crate::customization::{CustomizedGraph, CustomizedWeight};
use crate::server::api::Route;
use crate::server::error::{StatusCodeAndMessage, ToStatusResult};
use crate::server::names;
use crate::server::storage::S3Storage;
use crate::server::util::{
    spawn_blocking_status, stream_dataframe, stream_routes, ArrowIpcChunkStream,
};
use crate::weight::Weight;
use crate::ServerConfig;

pub struct H3ShortestPathParameters<W: Send + Sync> {
    graph: CustomizedGraph<W>,
    options: super::api::generated::ShortestPathOptions,
    origin_cells: Vec<H3Cell>,
    origin_dataframe: Option<H3DataFrame>,
    destination_cells: Vec<H3Cell>,
    destination_dataframe: Option<H3DataFrame>,
}

pub async fn create_parameters<W: Send + Sync>(
    request: super::api::generated::H3ShortestPathRequest,
    storage: Arc<S3Storage<W>>,
    config: Arc<ServerConfig>,
) -> Result<H3ShortestPathParameters<W>, Status>
where
    W: Serialize + DeserializeOwned,
{
    let routing_mode = config.get_routing_mode(&request.routing_mode)?;
    let graph = storage
        .load_graph_from_option(&request.graph_handle)
        .await
        .map(|(graph, _)| {
            let mut cg = CustomizedGraph::from(graph);
            cg.set_routing_mode(routing_mode);
            cg
        })?;

    let (origin_cells, origin_dataframe) = storage
        .load_cell_selection(
            request
                .origins
                .as_ref()
                .ok_or_else(|| Status::invalid_argument("origins not set"))?,
            graph.h3_resolution(),
        )
        .await?;

    let (destination_cells, destination_dataframe) = storage
        .load_cell_selection(
            request
                .destinations
                .as_ref()
                .ok_or_else(|| Status::invalid_argument("destinations not set"))?,
            graph.h3_resolution(),
        )
        .await?;

    Ok(H3ShortestPathParameters {
        graph,
        options: request.options.unwrap_or_default(),
        origin_cells,
        origin_dataframe,
        destination_cells,
        destination_dataframe,
    })
}

async fn spawn_h3_shortest_path<F, R, E>(func: F) -> Result<R, Status>
where
    F: FnOnce() -> Result<R, E> + Send + 'static,
    E: Debug + Send + 'static + StatusCodeAndMessage,
    R: Send + 'static,
{
    spawn_blocking_status(func).await?.to_status_result()
}

pub async fn h3_shortest_path<W: 'static + Send + Sync>(
    parameters: H3ShortestPathParameters<W>,
) -> Result<Response<ArrowIpcChunkStream>, Status>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
{
    stream_dataframe(
        uuid::Uuid::new_v4().to_string(),
        spawn_h3_shortest_path(move || h3_shortest_path_internal(parameters)).await?,
    )
    .await
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
struct PathSummary<W> {
    cost: W,
    path_length_m: OrderedFloat<f64>,
    destination_cell: H3Cell,
}

impl<W> TryFrom<Path<W>> for PathSummary<W>
where
    W: Copy,
{
    type Error = h3ron_graph::Error;

    fn try_from(path: Path<W>) -> Result<Self, Self::Error> {
        let mut path_length_m = 0.0;
        for edge in path.directed_edge_path.edges() {
            path_length_m += edge.exact_length_m()?;
        }
        Ok(Self {
            cost: path.cost,
            path_length_m: path_length_m.into(),
            destination_cell: path.destination_cell,
        })
    }
}

fn h3_shortest_path_internal<W: Send + Sync>(
    parameters: H3ShortestPathParameters<W>,
) -> eyre::Result<DataFrame>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
{
    let pathmap = parameters.graph.shortest_path_many_to_many_map(
        &parameters.origin_cells,
        &parameters.destination_cells,
        &parameters.options,
        PathSummary::try_from,
    )?;

    let mut shortest_path_df = {
        let capacity = pathmap.len()
            * parameters
                .options
                .num_destinations_to_reach()
                .unwrap_or(parameters.destination_cells.len());

        let mut origin_cell_vec = Vec::with_capacity(capacity);
        let mut destination_cell_vec = Vec::with_capacity(capacity);
        let mut path_cell_length_m_vec = Vec::with_capacity(capacity);
        let mut travel_duration_secs_vec = Vec::with_capacity(capacity);
        let mut edge_preferences_vec = Vec::with_capacity(capacity);

        for (origin_cell, paths) in pathmap.iter() {
            if paths.is_empty() {
                // keep one entry for the origin regardless if a route to a
                // destination was found.

                origin_cell_vec.push(origin_cell.h3index() as u64);
                destination_cell_vec.push(None);
                path_cell_length_m_vec.push(None);
                travel_duration_secs_vec.push(None);
                edge_preferences_vec.push(None);
            } else {
                for path_summary in paths.iter() {
                    origin_cell_vec.push(origin_cell.h3index() as u64);
                    destination_cell_vec.push(Some(path_summary.destination_cell.h3index() as u64));
                    path_cell_length_m_vec.push(Some(path_summary.path_length_m.into_inner()));
                    travel_duration_secs_vec.push(Some(
                        path_summary.cost.travel_duration().get::<second>() as f32,
                    ));
                    edge_preferences_vec.push(Some(path_summary.cost.edge_preference()));
                }
            }
        }
        DataFrame::new(vec![
            Series::new(names::COL_H3INDEX_ORIGIN, origin_cell_vec),
            Series::new(names::COL_H3INDEX_DESTINATION, destination_cell_vec),
            Series::new(names::COL_PATH_LENGTH_METERS, path_cell_length_m_vec),
            Series::new(names::COL_TRAVEL_DURATION_SECS, travel_duration_secs_vec),
            Series::new(names::COL_EDGE_PREFERENCE, edge_preferences_vec),
        ])?
    };

    if let Some(origin_h3df) = parameters.origin_dataframe {
        inner_join_h3dataframe(
            &mut shortest_path_df,
            names::COL_H3INDEX_ORIGIN,
            origin_h3df,
            "origin_",
        )?;
    }

    if let Some(destination_h3df) = parameters.destination_dataframe {
        inner_join_h3dataframe(
            &mut shortest_path_df,
            names::COL_H3INDEX_DESTINATION,
            destination_h3df,
            "dest_",
        )?;
    }

    Ok(shortest_path_df)
}

pub async fn h3_shortest_path_routes<W: 'static + Send + Sync, R, F, E>(
    parameters: H3ShortestPathParameters<W>,
    transformer: F,
) -> Result<Response<ReceiverStream<Result<R, Status>>>, Status>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
    R: Route + Send + 'static,
    E: Debug + Send + 'static + StatusCodeAndMessage,
    F: FnMut(Path<CustomizedWeight<W>>) -> Result<R, E> + Send + 'static,
{
    let routes = spawn_h3_shortest_path(move || {
        parameters
            .graph
            .shortest_path_many_to_many(
                &parameters.origin_cells,
                &parameters.destination_cells,
                &parameters.options,
            )
            .map(|mut pathmap| {
                pathmap
                    .drain()
                    .flat_map(|(_k, v)| v)
                    .map(transformer)
                    .collect::<Result<Vec<_>, _>>()
                    .to_status_result()
            })
    })
    .await??;
    stream_routes(routes).await
}
