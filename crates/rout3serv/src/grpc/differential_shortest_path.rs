use ahash::RandomState;
use std::cmp::max;
use std::sync::Arc;

use geo_types::Coord;
use h3o::{CellIndex, LatLng, Resolution};
use hexigraph::algorithm::graph::differential_shortest_path::ExclusionDiff;
use hexigraph::algorithm::graph::path::Path;
use hexigraph::algorithm::graph::DifferentialShortestPath;
use hexigraph::algorithm::resolution::transform_resolution;
use hexigraph::container::treemap::H3Treemap;
use hexigraph::container::CellSet;
use hexigraph::graph::PreparedH3EdgeGraph;
use hexigraph::HasH3Resolution;
use polars::prelude::{DataFrame, DataFrameJoinOps, JoinType, NamedFrom, Series};
use polars_core::prelude::JoinArgs;
use serde::{Deserialize, Serialize};
use tonic::{Code, Status};
use tracing::Level;
use uom::si::time::second;

use crate::grpc::api::generated::{
    DifferentialShortestPathRequest, DifferentialShortestPathRoutes, RouteWkb, ShortestPathOptions,
};
use crate::grpc::error::{logged_status, StatusCodeAndMessage, ToStatusResult};
use crate::grpc::geometry::{buffer_meters, from_wkb, geom_to_h3};
use crate::grpc::util::{change_cell_resolution_dedup, StrId};
use crate::grpc::ServerImpl;
use crate::io::dataframe::CellDataFrame;
use crate::io::memory_cache::FetchError;
use crate::weight::{StandardWeight, Weight};

pub struct DspInput {
    /// the cells within the disturbance
    pub disturbance: H3Treemap<CellIndex>,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<CellIndex>,

    /// the destination cells to route to
    pub destinations: Vec<CellIndex>,

    pub store_output: bool,
    pub options: ShortestPathOptions,
    pub graph: Arc<PreparedH3EdgeGraph<StandardWeight>>,

    /// Setting a `downsampled_graph` will allow performing an initial routing at a lower resolution
    /// to reduce the number of routings to perform on the full-resolution graph. This has the potential
    /// to skew the results as a reduction in resolution may change the graph topology, but decreases the
    /// running time in most cases.
    /// The reduction should be no more than two resolutions.
    pub downsampled_graph: Option<Arc<PreparedH3EdgeGraph<StandardWeight>>>,
    pub ref_dataframe: CellDataFrame,
    pub ref_dataframe_cells: CellSet,
}

/// collect/prepare/download all input data needed for the differential shortest path
pub(crate) async fn collect_input(
    mut request: DifferentialShortestPathRequest,
    server_impl: &ServerImpl,
) -> Result<DspInput, Status> {
    let (graph, graph_key) = server_impl
        .retrieve_graph_by_handle(&request.graph_handle)
        .await?;

    let downsampled_graph = if request.downsampled_prerouting {
        // attempt to find a suitable graph at a lower resolution

        let mut downsampled_graph = None;
        let r_end: u8 = graph_key.h3_resolution.into();
        let r_start: u8 = r_end.saturating_sub(4);

        for r in (r_start..r_end).rev() {
            let mut gck = graph_key.clone();
            gck.h3_resolution = r.try_into().expect("resolution search");

            match server_impl.storage.retrieve_graph(gck).await {
                Ok(graph) => {
                    downsampled_graph = Some(graph);
                    break;
                }
                Err(FetchError::Fetch(e)) => {
                    if e.is_not_found() {
                        continue;
                    } else {
                        return Err(e.status());
                    }
                }
                Err(e) => return Err(e.status()),
            }
        }

        if downsampled_graph.is_none() {
            return Err(logged_status!(
                "no suitable graph at a lower resolution found",
                Code::InvalidArgument,
                Level::DEBUG
            ));
        }
        downsampled_graph
    } else {
        None
    };

    let (disturbance, within_buffer) = {
        let disturbance_wkb_geometry = std::mem::take(&mut request.disturbance_wkb_geometry);
        let radius_meters = request.radius_meters;
        tokio::task::block_in_place(|| {
            disturbance_and_buffered_cells(
                graph.h3_resolution(),
                &disturbance_wkb_geometry,
                radius_meters,
            )
        })?
    };

    let ref_dataframe = server_impl
        .storage
        .retrieve_dataframe(
            server_impl.dataset_by_name(&request.ref_dataset_name)?,
            &within_buffer,
            graph.h3_resolution(),
        )
        .await
        .to_status_result()?
        .ok_or_else(|| logged_status!("ref_dataset was empty", Code::NotFound, Level::WARN))?;

    let ref_dataframe_cells: CellSet = ref_dataframe
        .cell_u64s()
        .to_status_result()?
        .into_iter()
        .filter_map(|v| v.map(|h3index| CellIndex::try_from(h3index).ok()))
        .flatten()
        .collect();

    Ok(DspInput {
        disturbance,
        within_buffer,
        destinations: destination_cells(request.destinations, graph.h3_resolution())?,
        store_output: request.store_output,
        options: request.options.unwrap_or_default(),
        graph,
        downsampled_graph,
        ref_dataframe,
        ref_dataframe_cells,
    })
}

/// cells to route to
fn destination_cells(
    destinations: Vec<super::api::generated::Point>,
    h3_resolution: Resolution,
) -> Result<Vec<CellIndex>, Status> {
    let mut destination_cells = destinations
        .iter()
        .map(|pt| LatLng::try_from(Coord::from((pt.x, pt.y))).map(|ll| ll.to_cell(h3_resolution)))
        .collect::<Result<Vec<_>, _>>()
        .to_status_result_with_message(Code::Internal, || {
            "can not convert the target points to h3".to_string()
        })?;
    destination_cells.sort_unstable();
    destination_cells.dedup();
    Ok(destination_cells)
}

fn disturbance_and_buffered_cells(
    h3_resolution: Resolution,
    disturbance_wkb_geometry: &[u8],
    radius_meters: f64,
) -> Result<(H3Treemap<CellIndex>, Vec<CellIndex>), Status> {
    let disturbance_geom = from_wkb(disturbance_wkb_geometry)?;
    let disturbed_cells: H3Treemap<CellIndex> =
        H3Treemap::from_iter(geom_to_h3(disturbance_geom.clone(), h3_resolution, true)?);

    let buffered_cells = geom_to_h3(
        buffer_meters(&disturbance_geom, radius_meters)?,
        h3_resolution,
        true,
    )?;
    Ok((disturbed_cells, buffered_cells))
}

#[derive(Serialize, Deserialize)]
pub struct DspOutput {
    pub object_id: String,
    pub ref_dataframe: CellDataFrame,
    pub ref_dataframe_cells: CellSet,

    /// tuple: (origin h3 cell, diff)
    pub differential_shortest_paths: Vec<(CellIndex, ExclusionDiff<Path<StandardWeight>>)>,
}

impl StrId for DspOutput {
    fn id(&self) -> &str {
        self.object_id.as_ref()
    }
}

///
///
pub fn calculate(input: DspInput) -> Result<DspOutput, Status> {
    let origin_cells: Vec<CellIndex> = {
        let origin_cells: Vec<CellIndex> = {
            let mut origin_cells = Vec::with_capacity(input.within_buffer.len());
            for cell in &input.within_buffer {
                // exclude the cells of the disturbance itself as well as all origin cells without
                // any population from routing
                if input.ref_dataframe_cells.contains(cell) && !input.disturbance.contains(cell) {
                    origin_cells.push(*cell);
                }
            }
            origin_cells
        };

        if let Some(downsampled_graph) = input.downsampled_graph {
            let origin_cells_ds =
                change_cell_resolution_dedup(&origin_cells, downsampled_graph.h3_resolution());

            let destinations_ds = change_cell_resolution_dedup(
                &input.destinations,
                downsampled_graph.h3_resolution(),
            );

            let disturbance_ds: H3Treemap<_> = H3Treemap::from_iter(transform_resolution(
                input.disturbance.iter().filter_map(|v| v.ok()),
                downsampled_graph.h3_resolution(),
            ));

            let diff_ds = downsampled_graph
                .differential_shortest_path_map(
                    &origin_cells_ds,
                    &destinations_ds,
                    &disturbance_ds,
                    &input.options,
                    |path| Ok((path.cost, path.len())),
                )
                .to_status_result()?;

            // determinate the size of the k-ring to use to include enough full-resolution
            // cells around the found disturbance effect. This is essentially a buffering.
            let k_affected = max(
                1,
                (1500.0 / downsampled_graph.h3_resolution().edge_length_m()).ceil() as u32,
            );
            let mut affected_downsampled =
                CellSet::with_capacity_and_hasher(diff_ds.len(), RandomState::default());
            for cell in diff_ds.keys() {
                // the grid_disk creates essentially a buffer so the skew-effects of the
                // reduction of the resolution at the borders of the disturbance effect
                // are reduced. The result is a larger number of full-resolution routing runs
                // is performed.
                let disk: Vec<_> = cell.grid_disk(k_affected);

                if !disk.iter().all(|ring_cell| {
                    if let Some(diff) = diff_ds.get(ring_cell) {
                        diff.before_cell_exclusion == diff.after_cell_exclusion
                    } else {
                        true
                    }
                }) {
                    affected_downsampled.insert(*cell);
                }
            }

            let mut reduced_origin_cells = Vec::with_capacity(origin_cells.len());
            for cell in origin_cells {
                if let Some(parent_cell) = cell.parent(downsampled_graph.h3_resolution()) {
                    // always add cells within the downsampled disturbance to avoid ignoring cells directly
                    // bordering to the disturbance.
                    if affected_downsampled.contains(&parent_cell)
                        || disturbance_ds.contains(&parent_cell)
                    {
                        reduced_origin_cells.push(cell);
                    }
                }
            }
            reduced_origin_cells
        } else {
            origin_cells
        }
    };

    let diff: Vec<_> = input
        .graph
        .differential_shortest_path(
            &origin_cells,
            &input.destinations,
            &input.disturbance,
            &input.options,
        )
        .to_status_result_with_message(Code::Internal, || {
            "calculating differential_shortest_path failed".to_string()
        })?
        .into_iter()
        .collect();

    Ok(DspOutput {
        object_id: uuid::Uuid::new_v4().to_string(),
        ref_dataframe: input.ref_dataframe,
        ref_dataframe_cells: input.ref_dataframe_cells,
        differential_shortest_paths: diff,
    })
}

/// build an arrow dataset with some basic stats for each of the origin cells
fn disturbance_statistics_internal(output: &DspOutput) -> Result<DataFrame, Status> {
    let avg_travel_duration = |paths: &[Path<StandardWeight>]| -> Option<f64> {
        if paths.is_empty() {
            None
        } else {
            Some(
                paths
                    .iter()
                    .map(|p| p.cost.travel_duration().get::<second>() as f64)
                    .sum::<f64>()
                    / paths.len() as f64,
            )
        }
    };

    let avg_edge_preference = |paths: &[Path<StandardWeight>]| -> Option<f64> {
        if paths.is_empty() {
            None
        } else {
            Some(
                paths
                    .iter()
                    .map(|p| p.cost.edge_preference() as f64)
                    .sum::<f64>()
                    / paths.len() as f64,
            )
        }
    };

    let preferred_destination = |paths: &[Path<StandardWeight>]| -> Option<u64> {
        paths.first().map(|p| u64::from(p.destination_cell))
    };

    let mut cell_h3indexes = Vec::with_capacity(output.differential_shortest_paths.len());
    let mut num_reached_without_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut num_reached_with_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut avg_travel_duration_without_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut avg_travel_duration_with_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut avg_edge_preference_without_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut avg_edge_preference_with_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut preferred_destination_without_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    let mut preferred_destination_with_disturbance =
        Vec::with_capacity(output.differential_shortest_paths.len());
    for (origin_cell, diff) in &output.differential_shortest_paths {
        cell_h3indexes.push(u64::from(*origin_cell));
        //population_at_origin.push(output.population_at_origins.get(origin_cell).cloned());

        num_reached_without_disturbance.push(diff.before_cell_exclusion.len() as u64);
        avg_travel_duration_without_disturbance
            .push(avg_travel_duration(&diff.before_cell_exclusion));
        avg_edge_preference_without_disturbance
            .push(avg_edge_preference(&diff.before_cell_exclusion));

        num_reached_with_disturbance.push(diff.after_cell_exclusion.len() as u64);
        avg_travel_duration_with_disturbance.push(avg_travel_duration(&diff.after_cell_exclusion));
        avg_edge_preference_with_disturbance.push(avg_edge_preference(&diff.after_cell_exclusion));

        preferred_destination_without_disturbance
            .push(preferred_destination(&diff.before_cell_exclusion));
        preferred_destination_with_disturbance
            .push(preferred_destination(&diff.after_cell_exclusion));
    }

    let df = DataFrame::new(vec![
        Series::new("h3index_origin", &cell_h3indexes),
        Series::new(
            "preferred_dest_h3index_without_disturbance",
            &preferred_destination_without_disturbance,
        ),
        Series::new(
            "num_reached_without_disturbance",
            &num_reached_without_disturbance,
        ),
        Series::new(
            "avg_travel_duration_without_disturbance",
            &avg_travel_duration_without_disturbance,
        ),
        Series::new(
            "avg_edge_preference_without_disturbance",
            &avg_edge_preference_without_disturbance,
        ),
        Series::new(
            "preferred_dest_h3index_with_disturbance",
            &preferred_destination_with_disturbance,
        ),
        Series::new(
            "num_reached_with_disturbance",
            &num_reached_with_disturbance,
        ),
        Series::new(
            "avg_travel_duration_with_disturbance",
            &avg_travel_duration_with_disturbance,
        ),
        Series::new(
            "avg_edge_preference_with_disturbance",
            &avg_edge_preference_with_disturbance,
        ),
    ])
    .to_status_result()?;
    let df = df
        .join(
            &output.ref_dataframe.dataframe,
            ["h3index_origin"],
            [output.ref_dataframe.cell_column_name.as_str()],
            JoinArgs::new(JoinType::Inner),
        )
        .to_status_result()?;
    Ok(df)
}

pub fn disturbance_statistics(output: &DspOutput) -> Result<DataFrame, Status> {
    disturbance_statistics_internal(output)
}

pub fn build_routes_response(
    diff: &ExclusionDiff<Path<StandardWeight>>,
    smoothen_geometries: bool,
) -> Result<DifferentialShortestPathRoutes, Status> {
    let response = DifferentialShortestPathRoutes {
        routes_without_disturbance: diff
            .before_cell_exclusion
            .iter()
            .map(|path| RouteWkb::from_path(path, smoothen_geometries))
            .collect::<Result<_, _>>()?,
        routes_with_disturbance: diff
            .after_cell_exclusion
            .iter()
            .map(|path| RouteWkb::from_path(path, smoothen_geometries))
            .collect::<Result<_, _>>()?,
    };
    Ok(response)
}
