use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;

use h3ron::{H3Cell, HasH3Resolution, Index};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path::{ShortestPathManyToMany, ShortestPathOptions};
use h3ron_graph::graph::PreparedH3EdgeGraph;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use polars_core::prelude::{DataFrame, NamedFrom, Series};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uom::si::time::second;

use s3io::dataframe::{inner_join_h3dataframe, H3DataFrame};

use crate::server::api::Route;
use crate::server::names;
use crate::server::storage::S3Storage;
use crate::server::util::{
    spawn_blocking_status, stream_dataframe, stream_routes, ArrowRecordBatchStream,
};
use crate::weight::Weight;

pub struct H3ShortestPathParameters<W: Send + Sync> {
    graph: Arc<PreparedH3EdgeGraph<W>>,
    options: super::api::generated::ShortestPathOptions,
    origin_cells: Vec<H3Cell>,
    origin_dataframe: Option<H3DataFrame>,
    destination_cells: Vec<H3Cell>,
    destination_dataframe: Option<H3DataFrame>,
}

pub async fn create_parameters<W: Send + Sync>(
    request: super::api::generated::H3ShortestPathRequest,
    storage: Arc<S3Storage<W>>,
) -> Result<H3ShortestPathParameters<W>, Status>
where
    W: Serialize + DeserializeOwned,
{
    let (graph, _) = storage
        .load_graph_from_option(&request.graph_handle)
        .await?;

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
    E: Debug + Send + 'static,
    R: Send + 'static,
{
    Ok(spawn_blocking_status(func).await?.map_err(|e| {
        log::error!("calculating h3 shortest path failed: {:?}", e);
        Status::internal("calculating h3 shortest path failed")
    })?)
}

pub async fn h3_shortest_path<W: 'static + Send + Sync>(
    parameters: H3ShortestPathParameters<W>,
) -> Result<Response<ArrowRecordBatchStream>, Status>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
{
    stream_dataframe(
        uuid::Uuid::new_v4().to_string(),
        spawn_h3_shortest_path(move || h3_shortest_path_internal(parameters)).await?,
    )
    .await
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
        |p: Path<W>| {
            (
                *p.cost(),
                p.edges()
                    .iter()
                    .map(|edge| OrderedFloat::from(edge.cell_centroid_distance_m()))
                    .sum::<OrderedFloat<f64>>(),
                p.destination_cell().ok(),
            )
        },
    )?;

    let mut shortest_path_df = {
        let capacity = pathmap.len()
            * parameters
                .options
                .num_destinations_to_reach()
                .unwrap_or_else(|| parameters.destination_cells.len());

        let mut origin_cell_vec = Vec::with_capacity(capacity);
        let mut destination_cell_vec = Vec::with_capacity(capacity);
        let mut path_cell_length_m_vec = Vec::with_capacity(capacity);
        let mut travel_duration_secs_vec = Vec::with_capacity(capacity);
        let mut road_category_weight_vec = Vec::with_capacity(capacity);

        for (origin_cell, paths) in pathmap.iter() {
            if paths.is_empty() {
                // keep one entry for the origin regardless if a route to a
                // destination was found.

                origin_cell_vec.push(origin_cell.h3index() as u64);
                destination_cell_vec.push(None);
                path_cell_length_m_vec.push(None);
                travel_duration_secs_vec.push(None);
                road_category_weight_vec.push(None);
            } else {
                for (cost, path_length_dm, destination) in paths.iter() {
                    origin_cell_vec.push(origin_cell.h3index() as u64);
                    destination_cell_vec.push(destination.map(|c| c.h3index() as u64));
                    path_cell_length_m_vec.push(Some(path_length_dm.into_inner()));
                    travel_duration_secs_vec
                        .push(Some(cost.travel_duration().get::<second>() as f32));
                    road_category_weight_vec.push(Some(cost.category_weight()));
                }
            }
        }
        DataFrame::new(vec![
            Series::new(names::COL_H3INDEX_ORIGIN, origin_cell_vec),
            Series::new(names::COL_H3INDEX_DESTINATION, destination_cell_vec),
            Series::new(names::COL_PATH_LENGTH_METERS, path_cell_length_m_vec),
            Series::new(names::COL_TRAVEL_DURATION_SECS, travel_duration_secs_vec),
            Series::new(names::COL_ROAD_CATEGORY_WEIGHT, road_category_weight_vec),
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
    E: Debug + Send + 'static,
    F: FnMut(Path<W>) -> Result<R, E> + Send + 'static,
{
    let routes = spawn_blocking_status(move || {
        match parameters.graph.shortest_path_many_to_many(
            &parameters.origin_cells,
            &parameters.destination_cells,
            &parameters.options,
        ) {
            Ok(mut pathmap) => pathmap
                .drain()
                .flat_map(|(_k, v)| v)
                .map(transformer)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    log::error!("transforming routes failed: {:?}", e);
                    Status::internal("transforming routes failed")
                }),
            Err(e) => {
                log::error!("calculating h3 shortest path failed: {:?}", e);
                Err(Status::internal("calculating h3 shortest path failed"))
            }
        }
    })
    .await??;
    stream_routes(routes).await
}
