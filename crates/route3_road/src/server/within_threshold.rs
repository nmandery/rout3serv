use std::ops::Add;
use std::sync::Arc;

use h3ron::{H3Cell, HasH3Resolution, Index};
use h3ron_graph::algorithm::WithinWeightThresholdMany;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use num_traits::Zero;
use polars_core::prelude::{DataFrame, NamedFrom, Series};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tonic::{Response, Status};
use uom::si::f32::Time;
use uom::si::time::second;

use s3io::dataframe::{inner_join_h3dataframe, H3DataFrame};

use crate::server::storage::S3Storage;
use crate::server::util::{spawn_blocking_status, stream_dataframe, ArrowIpcChunkStream};
use crate::weight::Weight;

use super::names;

enum Threshold {
    TravelDuration(Time),
}

pub struct H3WithinThresholdParameters<W: Send + Sync> {
    graph: Arc<PreparedH3EdgeGraph<W>>,
    origin_cells: Vec<H3Cell>,
    origin_dataframe: Option<H3DataFrame>,
    threshold: Threshold,
}

pub async fn create_parameters<W: Send + Sync>(
    request: super::api::generated::H3WithinThresholdRequest,
    storage: Arc<S3Storage<W>>,
) -> Result<H3WithinThresholdParameters<W>, Status>
where
    W: Serialize + DeserializeOwned,
{
    let threshold = if request.travel_duration_secs_threshold.is_normal()
        && request.travel_duration_secs_threshold > 0.0
    {
        Threshold::TravelDuration(Time::new::<second>(request.travel_duration_secs_threshold))
    } else {
        return Err(Status::invalid_argument("invalid or no threshold given"));
    };

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

    Ok(H3WithinThresholdParameters {
        graph,
        origin_cells,
        origin_dataframe,
        threshold,
    })
}

pub async fn within_threshold<W: 'static + Send + Sync>(
    parameters: H3WithinThresholdParameters<W>,
) -> Result<Response<ArrowIpcChunkStream>, Status>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
{
    stream_dataframe(
        uuid::Uuid::new_v4().to_string(),
        spawn_blocking_status(move || within_threshold_internal(parameters))
            .await?
            .map_err(|e| {
                log::error!("calculating within threshold failed: {:?}", e);
                Status::internal("calculating within threshold failed")
            })?,
    )
    .await
}

fn within_threshold_internal<W: Send + Sync>(
    parameters: H3WithinThresholdParameters<W>,
) -> eyre::Result<DataFrame>
where
    W: Send + Sync + Ord + Copy + Add + Zero + Weight,
{
    let threshold_weight = match parameters.threshold {
        Threshold::TravelDuration(travel_duration) => W::from_travel_duration(travel_duration),
    };

    // use the minimum weight encountered
    let agg_fn = |existing: &mut W, new: W| {
        if new < *existing {
            *existing = new;
        }
    };

    let cellmap = parameters.graph.cells_within_weight_threshold_many(
        &parameters.origin_cells,
        threshold_weight,
        agg_fn,
    );

    let capacity = cellmap.len();
    let (cell_h3indexes, travel_duration_secs, road_category_weights) = cellmap.iter().fold(
        (
            Vec::with_capacity(capacity),
            Vec::with_capacity(capacity),
            Vec::with_capacity(capacity),
        ),
        |mut acc, item| {
            acc.0.push(item.0.h3index() as u64);
            acc.1.push(item.1.travel_duration().get::<second>() as f32);
            acc.2.push(item.1.category_weight());
            acc
        },
    );

    let mut df = DataFrame::new(vec![
        Series::new(names::COL_H3INDEX_ORIGIN, cell_h3indexes),
        Series::new(names::COL_TRAVEL_DURATION_SECS, travel_duration_secs),
        Series::new(names::COL_ROAD_CATEGORY_WEIGHT, road_category_weights),
    ])?;

    // join origin dataframe if there is any
    if let Some(origin_h3df) = parameters.origin_dataframe {
        inner_join_h3dataframe(&mut df, names::COL_H3INDEX_ORIGIN, origin_h3df, "origin_")?;
    }
    Ok(df)
}
