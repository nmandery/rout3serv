use std::cmp::max;
use std::ops::Add;
use std::sync::Arc;

use geo_types::Coordinate;
use h3ron::collections::{H3CellSet, H3Treemap, RandomState};
use h3ron::iter::change_resolution;
use h3ron::{H3Cell, H3DirectedEdge, HasH3Resolution, Index};
use h3ron_graph::algorithm::differential_shortest_path::{DifferentialShortestPath, ExclusionDiff};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use num_traits::Zero;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tonic::{Code, Status};
use uom::si::time::second;

use s3io::dataframe::H3DataFrame;
use s3io::polars_core::prelude::{DataFrame, JoinType, NamedFrom, Series};

use crate::io::graph_store::GraphCacheKey;
use crate::server::api::generated::{
    DifferentialShortestPathRequest, DifferentialShortestPathRoutes, RouteWkb, ShortestPathOptions,
};
use crate::server::error::ToStatusResult;
use crate::server::storage::S3Storage;
use crate::server::util::{change_cell_resolution_dedup, index_collection_from_h3dataframe, StrId};
use crate::server::vector::{buffer_meters, gdal_geom_to_h3, read_wkb_to_gdal};
use crate::weight::Weight;

pub struct DspInput<W: Send + Sync> {
    /// the cells within the disturbance
    pub disturbance: H3Treemap<H3Cell>,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the destination cells to route to
    pub destinations: Vec<H3Cell>,

    pub store_output: bool,
    pub options: ShortestPathOptions,
    pub graph: Arc<PreparedH3EdgeGraph<W>>,

    /// Setting a `downsampled_graph` will allow performing an initial routing at a lower resolution
    /// to reduce the number of routings to perform on the full-resolution graph. This has the potential
    /// to skew the results as a reduction in resolution may change the graph topology, but decreases the
    /// running time in most cases.
    /// The reduction should be no more than two resolutions.
    pub downsampled_graph: Option<Arc<PreparedH3EdgeGraph<W>>>,
    pub ref_dataframe: H3DataFrame,
    pub ref_dataframe_cells: H3CellSet,
}

/// collect/prepare/download all input data needed for the differential shortest path
pub async fn collect_input<W: Send + Sync>(
    mut request: DifferentialShortestPathRequest,
    storage: Arc<S3Storage<W>>,
) -> Result<DspInput<W>, Status>
where
    W: Serialize + DeserializeOwned,
{
    let (graph, graph_cache_key) = storage
        .load_graph_from_option(&request.graph_handle)
        .await?;

    // obtain the downsampled graph if requested
    let downsampled_graph = if request.downsampled_prerouting {
        // attempt to find a suitable graph at a lower resolution
        let mut found_gck: Option<GraphCacheKey> = None;
        for gck in storage.load_graph_cache_keys().await?.drain(..) {
            if gck.name == graph_cache_key.name && gck.h3_resolution < graph_cache_key.h3_resolution
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
            Some(storage.load_graph(&dsg_gck).await?)
        } else {
            return Err(Status::invalid_argument(
                "no suitable graph at a lower resolution found",
            ));
        }
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

    let ref_dataframe = storage
        .load_dataset_dataframe(
            storage.get_dataset_config(request.ref_dataset_name)?,
            &within_buffer,
            graph.h3_resolution(),
        )
        .await?;
    let ref_dataframe_cells: H3CellSet = index_collection_from_h3dataframe(&ref_dataframe)?;

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
    h3_resolution: u8,
) -> Result<Vec<H3Cell>, Status> {
    let mut destination_cells = destinations
        .iter()
        .map(|pt| H3Cell::from_coordinate(Coordinate::from((pt.x, pt.y)), h3_resolution))
        .collect::<Result<Vec<_>, _>>()
        .to_status_result_with_message(Code::Internal, || {
            "can not convert the target points to h3".to_string()
        })?;
    destination_cells.sort_unstable();
    destination_cells.dedup();
    Ok(destination_cells)
}

fn disturbance_and_buffered_cells(
    h3_resolution: u8,
    disturbance_wkb_geometry: &[u8],
    radius_meters: f64,
) -> Result<(H3Treemap<H3Cell>, Vec<H3Cell>), Status> {
    let disturbance_geom = read_wkb_to_gdal(disturbance_wkb_geometry)?;
    let disturbed_cells: H3Treemap<H3Cell> = H3Treemap::from_iter_with_sort(
        gdal_geom_to_h3(&disturbance_geom, h3_resolution, true)?.drain(..),
    );

    let buffered_cells = gdal_geom_to_h3(
        &buffer_meters(&disturbance_geom, radius_meters)?,
        h3_resolution,
        true,
    )?;
    Ok((disturbed_cells, buffered_cells))
}

#[derive(Serialize, Deserialize)]
pub struct DspOutput<W: Send + Sync> {
    pub object_id: String,
    pub ref_dataframe: H3DataFrame,
    pub ref_dataframe_cells: H3CellSet,

    /// tuple: (origin h3 cell, diff)
    pub differential_shortest_paths: Vec<(H3Cell, ExclusionDiff<Path<W>>)>,
}

impl<W: Send + Sync> StrId for DspOutput<W> {
    fn id(&self) -> &str {
        self.object_id.as_ref()
    }
}

///
///
pub fn calculate<W>(input: DspInput<W>) -> anyhow::Result<DspOutput<W>>
where
    W: PartialOrd + PartialEq + Add + Copy + Send + Ord + Zero + Sync,
{
    let origin_cells: Vec<H3Cell> = {
        let origin_cells: Vec<H3Cell> = {
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
                change_cell_resolution_dedup(&origin_cells, downsampled_graph.h3_resolution())?;

            let destinations_ds = change_cell_resolution_dedup(
                &input.destinations,
                downsampled_graph.h3_resolution(),
            )?;

            let disturbance_ds: H3Treemap<_> = H3Treemap::from_result_iter_with_sort(
                change_resolution(input.disturbance.iter(), downsampled_graph.h3_resolution()),
            )
            .to_status_result()?;

            let diff_ds = downsampled_graph.differential_shortest_path_map(
                &origin_cells_ds,
                &destinations_ds,
                &disturbance_ds,
                &input.options,
                |path| Ok((path.cost, path.len())),
            )?;

            // determinate the size of the k-ring to use to include enough full-resolution
            // cells around the found disturbance effect. This is essentially a buffering.
            let k_affected = max(
                1,
                (1500.0 / H3DirectedEdge::edge_length_avg_m(downsampled_graph.h3_resolution())?)
                    .ceil() as u32,
            );
            let mut affected_downsampled: H3CellSet =
                H3CellSet::with_capacity_and_hasher(diff_ds.len(), RandomState::default());
            for cell in diff_ds.keys() {
                // the grid_disk creates essentially a buffer so the skew-effects of the
                // reduction of the resolution at the borders of the disturbance effect
                // are reduced. The result is a larger number of full-resolution routing runs
                // is performed.
                if !cell.grid_disk(k_affected)?.iter().all(|ring_cell| {
                    if let Some(diff) = diff_ds.get(&ring_cell) {
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
                let parent_cell = cell.get_parent(downsampled_graph.h3_resolution())?;

                // always add cells within the downsampled disturbance to avoid ignoring cells directly
                // bordering to the disturbance.
                if affected_downsampled.contains(&parent_cell)
                    || disturbance_ds.contains(&parent_cell)
                {
                    reduced_origin_cells.push(cell);
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
        )?
        .drain()
        .collect();

    Ok(DspOutput {
        object_id: uuid::Uuid::new_v4().to_string(),
        ref_dataframe: input.ref_dataframe,
        ref_dataframe_cells: input.ref_dataframe_cells,
        differential_shortest_paths: diff,
    })
}

/// build an arrow dataset with some basic stats for each of the origin cells
fn disturbance_statistics_internal<W: Send + Sync>(
    output: &DspOutput<W>,
) -> anyhow::Result<DataFrame>
where
    W: Weight,
{
    let avg_travel_duration = |paths: &[Path<W>]| -> Option<f64> {
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

    let avg_edge_preference = |paths: &[Path<W>]| -> Option<f64> {
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

    let preferred_destination = |paths: &[Path<W>]| -> Option<u64> {
        paths.first().map(|p| p.destination_cell.h3index() as u64)
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
        cell_h3indexes.push(origin_cell.h3index() as u64);
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
    ])?;
    let df = df.join(
        &output.ref_dataframe.dataframe,
        ["h3index_origin"],
        [output.ref_dataframe.h3index_column_name.as_str()],
        JoinType::Inner,
        None,
    )?;
    Ok(df)
}

pub fn disturbance_statistics<W: Send + Sync>(output: &DspOutput<W>) -> Result<DataFrame, Status>
where
    W: Weight,
{
    disturbance_statistics_internal(output).to_status_result_with_message(Code::Internal, || {
        "calculating population movement stats failed".to_string()
    })
}

pub fn build_routes_response<W: Send + Sync>(
    diff: &ExclusionDiff<Path<W>>,
    smoothen_geometries: bool,
) -> Result<DifferentialShortestPathRoutes, Status>
where
    W: Weight,
{
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
