use h3ron::{HasH3Resolution, Index};
use h3ron_graph::algorithm::WithinWeightThresholdMany;
use polars::prelude::{DataFrame, NamedFrom, Series};
use tonic::{Code, Response, Status};
use tracing::log::Level;
use uom::si::f32::Time;
use uom::si::time::second;

use crate::customization::{CustomizedGraph, CustomizedWeight};
use crate::grpc::error::{logged_status, ToStatusResult};
use crate::grpc::util::{
    inner_join_h3dataframe, spawn_blocking_status, stream_dataframe, ArrowIpcChunkStream,
};
use crate::grpc::{LoadedCellSelection, ServerImpl, ServerWeight};
use crate::weight::Weight;

use super::names;

pub enum Threshold {
    TravelDuration(Time),
}

pub struct H3WithinThresholdParameters<W: ServerWeight> {
    pub graph: CustomizedGraph<W>,
    pub origins: LoadedCellSelection,
    pub threshold: Threshold,
}

pub(crate) async fn create_parameters<W: ServerWeight>(
    request: super::api::generated::H3WithinThresholdRequest,
    server_impl: &ServerImpl<W>,
) -> Result<H3WithinThresholdParameters<W>, Status> {
    let threshold = if request.travel_duration_secs_threshold.is_normal()
        && request.travel_duration_secs_threshold > 0.0
    {
        Threshold::TravelDuration(Time::new::<second>(request.travel_duration_secs_threshold))
    } else {
        return Err(logged_status(
            "invalid or no threshold given",
            Code::InvalidArgument,
            Level::Debug,
        ));
    };
    let routing_mode = server_impl.config.get_routing_mode(&request.routing_mode)?;
    let graph = server_impl
        .retrieve_graph_by_handle(&request.graph_handle)
        .await
        .map(|(graph, _)| {
            let mut cg = CustomizedGraph::from(graph);
            cg.set_routing_mode(routing_mode);
            cg
        })?;

    let origins = server_impl
        .load_cell_selection(&request.origins, graph.h3_resolution(), "origins")
        .await?;

    Ok(H3WithinThresholdParameters {
        graph,
        origins,
        threshold,
    })
}

pub async fn within_threshold<W: 'static + ServerWeight>(
    parameters: H3WithinThresholdParameters<W>,
) -> Result<Response<ArrowIpcChunkStream>, Status> {
    stream_dataframe(
        uuid::Uuid::new_v4().to_string(),
        spawn_blocking_status(move || within_threshold_internal(parameters))
            .await?
            .to_status_result_with_message(Code::Internal, || {
                "calculating within threshold failed".to_string()
            })?,
    )
    .await
}

fn within_threshold_internal<W: ServerWeight>(
    parameters: H3WithinThresholdParameters<W>,
) -> Result<DataFrame, Status> {
    let threshold_weight = match parameters.threshold {
        Threshold::TravelDuration(travel_duration) => {
            CustomizedWeight::<W>::from_travel_duration(travel_duration)
        }
    };

    // use the minimum weight encountered
    let agg_fn = |existing: &mut CustomizedWeight<W>, new: CustomizedWeight<W>| {
        if new < *existing {
            *existing = new;
        }
    };

    let cellmap = parameters
        .graph
        .cells_within_weight_threshold_many(&parameters.origins.cells, threshold_weight, agg_fn)
        .to_status_result_with_message(Code::Internal, || {
            "isolating cells within threshold failed".to_string()
        })?;

    let capacity = cellmap.len();
    let (cell_h3indexes, travel_duration_secs, edge_preferences) = cellmap.iter().fold(
        (
            Vec::with_capacity(capacity),
            Vec::with_capacity(capacity),
            Vec::with_capacity(capacity),
        ),
        |mut acc, item| {
            acc.0.push(item.0.h3index());
            acc.1.push(item.1.travel_duration().get::<second>());
            acc.2.push(item.1.edge_preference());
            acc
        },
    );

    let mut df = DataFrame::new(vec![
        Series::new(names::COL_H3INDEX_ORIGIN, cell_h3indexes),
        Series::new(names::COL_TRAVEL_DURATION_SECS, travel_duration_secs),
        Series::new(names::COL_EDGE_PREFERENCE, edge_preferences),
    ])
    .to_status_result()?;

    // join origin dataframe if there is any
    if let Some(origin_h3df) = parameters.origins.dataframe {
        inner_join_h3dataframe(&mut df, names::COL_H3INDEX_ORIGIN, origin_h3df, "origin_")?;
    }
    Ok(df)
}
