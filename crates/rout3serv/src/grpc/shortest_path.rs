use std::fmt::Debug;

use h3ron::{H3Cell, HasH3Resolution, Index};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path::ShortestPathOptions;
use h3ron_graph::algorithm::ShortestPathManyToMany;
use ordered_float::OrderedFloat;
use polars::prelude::{DataFrame, NamedFrom, Series};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uom::si::time::second;

use crate::customization::{CustomizedGraph, CustomizedWeight};
use crate::grpc::api::Route;
use crate::grpc::error::{StatusCodeAndMessage, ToStatusResult};
use crate::grpc::util::{
    inner_join_h3dataframe, spawn_blocking_status, stream_dataframe, stream_routes,
    ArrowIpcChunkStream,
};
use crate::grpc::{names, LoadedCellSelection, ServerImpl};
use crate::weight::Weight;

pub struct H3ShortestPathParameters {
    graph: CustomizedGraph,
    options: super::api::generated::ShortestPathOptions,
    origins: LoadedCellSelection,
    destinations: LoadedCellSelection,
}

pub(crate) async fn create_parameters(
    request: super::api::generated::H3ShortestPathRequest,
    server_impl: &ServerImpl,
) -> Result<H3ShortestPathParameters, Status> {
    let routing_mode = server_impl.config.get_routing_mode(&request.routing_mode)?;
    let graph = server_impl
        .retrieve_graph_by_handle(&request.graph_handle)
        .await
        .map(|(graph, _)| {
            let mut cg = CustomizedGraph::from(graph);
            cg.set_routing_mode(routing_mode);
            cg
        })
        .to_status_result()?;

    let origins = server_impl
        .load_cell_selection(&request.origins, graph.h3_resolution(), "origins")
        .await?;

    let destinations = server_impl
        .load_cell_selection(&request.destinations, graph.h3_resolution(), "destinations")
        .await?;

    Ok(H3ShortestPathParameters {
        graph,
        options: request.options.unwrap_or_default(),
        origins,
        destinations,
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

pub async fn h3_shortest_path(
    parameters: H3ShortestPathParameters,
) -> Result<Response<ArrowIpcChunkStream>, Status> {
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
            path_length_m += edge.length_m()?;
        }
        Ok(Self {
            cost: path.cost,
            path_length_m: path_length_m.into(),
            destination_cell: path.destination_cell,
        })
    }
}

fn h3_shortest_path_internal(parameters: H3ShortestPathParameters) -> Result<DataFrame, Status> {
    let pathmap = parameters
        .graph
        .shortest_path_many_to_many_map(
            &parameters.origins.cells,
            &parameters.destinations.cells,
            &parameters.options,
            PathSummary::try_from,
        )
        .to_status_result()?;

    let mut shortest_path_df = {
        let capacity = pathmap.len()
            * parameters
                .options
                .num_destinations_to_reach()
                .unwrap_or(parameters.destinations.cells.len());

        let mut origin_cell_vec = Vec::with_capacity(capacity);
        let mut destination_cell_vec = Vec::with_capacity(capacity);
        let mut path_cell_length_m_vec = Vec::with_capacity(capacity);
        let mut travel_duration_secs_vec = Vec::with_capacity(capacity);
        let mut edge_preferences_vec = Vec::with_capacity(capacity);

        for (origin_cell, paths) in pathmap.iter() {
            if paths.is_empty() {
                // keep one entry for the origin regardless if a route to a
                // destination was found.

                origin_cell_vec.push(origin_cell.h3index());
                destination_cell_vec.push(None);
                path_cell_length_m_vec.push(None);
                travel_duration_secs_vec.push(None);
                edge_preferences_vec.push(None);
            } else {
                for path_summary in paths.iter() {
                    origin_cell_vec.push(origin_cell.h3index());
                    destination_cell_vec.push(Some(path_summary.destination_cell.h3index()));
                    path_cell_length_m_vec.push(Some(path_summary.path_length_m.into_inner()));
                    travel_duration_secs_vec
                        .push(Some(path_summary.cost.travel_duration().get::<second>()));
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
        ])
        .to_status_result()?
    };

    if let Some(origin_h3df) = parameters.origins.dataframe {
        inner_join_h3dataframe(
            &mut shortest_path_df,
            names::COL_H3INDEX_ORIGIN,
            origin_h3df,
            "origin_",
        )?;
    }

    if let Some(destination_h3df) = parameters.destinations.dataframe {
        inner_join_h3dataframe(
            &mut shortest_path_df,
            names::COL_H3INDEX_DESTINATION,
            destination_h3df,
            "dest_",
        )?;
    }

    Ok(shortest_path_df)
}

pub async fn h3_shortest_path_routes<R, F, E>(
    parameters: H3ShortestPathParameters,
    transformer: F,
) -> Result<Response<ReceiverStream<Result<R, Status>>>, Status>
where
    R: Route + Send + 'static,
    E: Debug + Send + 'static + StatusCodeAndMessage,
    F: FnMut(Path<CustomizedWeight>) -> Result<R, E> + Send + 'static,
{
    let routes = spawn_h3_shortest_path(move || {
        parameters
            .graph
            .shortest_path_many_to_many(
                &parameters.origins.cells,
                &parameters.destinations.cells,
                &parameters.options,
            )
            .map(|pathmap| {
                pathmap
                    .into_iter()
                    .flat_map(|(_k, v)| v)
                    .map(transformer)
                    .collect::<Result<Vec<_>, _>>()
                    .to_status_result()
            })
    })
    .await??;
    stream_routes(routes).await
}
