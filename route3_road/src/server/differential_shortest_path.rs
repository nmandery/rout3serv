use std::cmp::max;
use std::sync::Arc;

use arrow2::array::{Float32Vec, Float64Vec, UInt64Vec};
use arrow2::datatypes::{DataType, Field, Schema};
use arrow2::record_batch::RecordBatch;
use eyre::Result;
use geo_types::Coordinate;
use h3ron::collections::{H3CellMap, H3CellSet, H3Treemap};
use h3ron::iter::change_cell_resolution;
use h3ron::{H3Cell, H3Edge, HasH3Resolution, Index};
use h3ron_graph::algorithm::differential_shortest_path::{DifferentialShortestPath, ExclusionDiff};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tonic::Status;

use crate::server::api::generated::{
    DifferentialShortestPathRequest, DifferentialShortestPathRoutes, RouteWkb, ShortestPathOptions,
};
use crate::server::util::StrId;
use crate::server::vector::{buffer_meters, gdal_geom_to_h3, read_wkb_to_gdal};
use crate::weight::Weight;

pub struct DspInput {
    /// the cells within the disturbance
    pub disturbance: H3Treemap<H3Cell>,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the destination cells to route to
    pub destinations: Vec<H3Cell>,

    pub downsampled_prerouting: bool,
    pub store_output: bool,
    pub options: ShortestPathOptions,
}

impl DifferentialShortestPathRequest {
    pub fn get_input(&self, h3_resolution: u8) -> std::result::Result<DspInput, Status> {
        let (disturbance, within_buffer) = self.disturbance_and_buffered_cells(h3_resolution)?;
        Ok(DspInput {
            disturbance,
            within_buffer,
            destinations: self.destination_cells(h3_resolution)?,
            downsampled_prerouting: self.downsampled_prerouting,
            store_output: self.store_output,
            options: self.options.clone().unwrap_or_else(Default::default),
        })
    }

    fn disturbance_and_buffered_cells(
        &self,
        h3_resolution: u8,
    ) -> std::result::Result<(H3Treemap<H3Cell>, Vec<H3Cell>), Status> {
        let disturbance_geom = read_wkb_to_gdal(&self.disturbance_wkb_geometry)?;
        let disturbed_cells: H3Treemap<H3Cell> =
            gdal_geom_to_h3(&disturbance_geom, h3_resolution, true)?
                .drain()
                .collect();

        let buffered_cells: Vec<_> = gdal_geom_to_h3(
            &buffer_meters(&disturbance_geom, self.radius_meters)?,
            h3_resolution,
            true,
        )?
        .drain()
        .collect();
        Ok((disturbed_cells, buffered_cells))
    }

    /// cells to route to
    fn destination_cells(&self, h3_resolution: u8) -> std::result::Result<Vec<H3Cell>, Status> {
        let mut destination_cells = self
            .destinations
            .iter()
            .map(|pt| H3Cell::from_coordinate(&Coordinate::from((pt.x, pt.y)), h3_resolution))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                log::error!("can not convert the target_points to h3: {:?}", e);
                Status::internal("can not convert the target_points to h3")
            })?;
        destination_cells.sort_unstable();
        destination_cells.dedup();
        Ok(destination_cells)
    }
}

#[derive(Serialize, Deserialize)]
pub struct DspOutput {
    pub object_id: String,
    pub population_at_origins: H3CellMap<f32>,

    /// tuple: (origin h3 cell, diff)
    pub differential_shortest_paths: Vec<(H3Cell, ExclusionDiff<Path<Weight>>)>,
}

impl StrId for DspOutput {
    fn id(&self) -> &str {
        self.object_id.as_ref()
    }
}

///
///
/// Setting a `downsampled_routing_graph` will allow performing an initial routing at a lower resolution
/// to reduce the number of routings to perform on the full-resolution graph. This has the potential
/// to skew the results as a reduction in resolution may change the graph topology, but decreases the
/// running time in most cases.
/// The reduction should be no more than two resolutions.
pub fn calculate(
    prepared_graph: Arc<PreparedH3EdgeGraph<Weight>>,
    input: DspInput,
    population: H3CellMap<f32>,
    downsampled_graph: Option<Arc<PreparedH3EdgeGraph<Weight>>>,
) -> Result<DspOutput> {
    let mut population_at_origins: H3CellMap<f32> = H3CellMap::default();

    let origin_cells: Vec<H3Cell> = {
        let origin_cells: Vec<H3Cell> = {
            let mut origin_cells = Vec::with_capacity(input.within_buffer.len());
            for cell in input.within_buffer.iter() {
                // exclude the cells of the disturbance itself as well as all origin cells without
                // any population from routing
                if let (Some(pop), false) = (population.get(cell), input.disturbance.contains(cell))
                {
                    population_at_origins.insert(*cell, *pop);
                    origin_cells.push(*cell);
                }
            }
            origin_cells
        };

        if let Some(downsampled_graph) = downsampled_graph {
            let mut origin_cells_ds: Vec<_> =
                change_cell_resolution(&origin_cells, downsampled_graph.h3_resolution()).collect();
            origin_cells_ds.sort_unstable();
            origin_cells_ds.dedup();

            let mut destinations_ds: Vec<_> =
                change_cell_resolution(&input.destinations, downsampled_graph.h3_resolution())
                    .collect();
            destinations_ds.sort_unstable();
            destinations_ds.dedup();

            let disturbance_ds: H3Treemap<_> =
                change_cell_resolution(input.disturbance.iter(), downsampled_graph.h3_resolution())
                    .collect();

            let diff_ds = downsampled_graph.differential_shortest_path_map(
                &origin_cells_ds,
                &destinations_ds,
                &disturbance_ds,
                &input.options,
                |path| (path.cost, path.len()),
            )?;

            // determinate the size of the k-ring to use to include enough full-resolution
            // cells around the found disturbance effect. This is essentially a buffering.
            let k_affected = max(
                1,
                (1500.0 / H3Edge::edge_length_m(downsampled_graph.h3_resolution())).ceil() as u32,
            );
            let affected_downsampled: H3CellSet = diff_ds
                .par_keys()
                .filter(|cell| {
                    // the k_ring creates essentially a buffer so the skew-effects of the
                    // reduction of the resolution at the borders of the disturbance effect
                    // are reduced. The result is a larger number of full-resolution routing runs
                    // is performed.
                    !cell.k_ring(k_affected).iter().all(|ring_cell| {
                        if let Some(diff) = diff_ds.get(&ring_cell) {
                            diff.before_cell_exclusion == diff.after_cell_exclusion
                        } else {
                            true
                        }
                    })
                })
                .copied()
                .collect();

            origin_cells
                .iter()
                .filter(|cell| {
                    let parent_cell = cell.get_parent_unchecked(downsampled_graph.h3_resolution());
                    // always add cells within the downsampled disturbance to avoid ignoring cells directly
                    // bordering to the disturbance.
                    affected_downsampled.contains(&parent_cell)
                        || disturbance_ds.contains(&parent_cell)
                })
                .copied()
                .collect()
        } else {
            origin_cells
        }
    };

    let diff: Vec<_> = prepared_graph
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
        population_at_origins,
        differential_shortest_paths: diff,
    })
}

/// build an arrow dataset with some basic stats for each of the origin cells
pub fn disturbance_statistics(output: &DspOutput) -> Result<Vec<RecordBatch>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("h3index_origin", DataType::UInt64, false),
        Field::new(
            "preferred_dest_h3index_without_disturbance",
            DataType::UInt64,
            true,
        ),
        Field::new(
            "preferred_dest_h3index_with_disturbance",
            DataType::UInt64,
            true,
        ),
        Field::new("population_origin", DataType::Float32, true),
        Field::new("num_reached_without_disturbance", DataType::UInt64, false),
        Field::new("num_reached_with_disturbance", DataType::UInt64, false),
        Field::new("avg_cost_without_disturbance", DataType::Float64, true),
        Field::new("avg_cost_with_disturbance", DataType::Float64, true),
    ]));

    let avg_cost = |paths: &[Path<Weight>]| -> Option<f64> {
        if paths.is_empty() {
            None
        } else {
            Some(paths.iter().map(|p| f64::from(p.cost)).sum::<f64>() / paths.len() as f64)
        }
    };

    let preferred_destination = |paths: &[Path<Weight>]| -> Option<u64> {
        paths
            .first()
            .map(|p| p.destination_cell().ok())
            .flatten()
            .map(|cell| cell.h3index() as u64)
    };

    let mut batches = vec![];
    let chunk_size = 2000_usize;
    for chunk in &output.differential_shortest_paths.iter().chunks(chunk_size) {
        let mut cell_h3indexes = UInt64Vec::with_capacity(chunk_size);
        let mut population_at_origin = Float32Vec::with_capacity(chunk_size);
        let mut num_reached_without_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut num_reached_with_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut avg_cost_without_disturbance = Float64Vec::with_capacity(chunk_size);
        let mut avg_cost_with_disturbance = Float64Vec::with_capacity(chunk_size);
        let mut preferred_destination_without_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut preferred_destination_with_disturbance = UInt64Vec::with_capacity(chunk_size);
        for (origin_cell, diff) in chunk {
            cell_h3indexes.push(Some(origin_cell.h3index() as u64));
            population_at_origin.push(output.population_at_origins.get(origin_cell).cloned());

            num_reached_without_disturbance.push(Some(diff.before_cell_exclusion.len() as u64));
            avg_cost_without_disturbance.push(avg_cost(&diff.before_cell_exclusion));

            num_reached_with_disturbance.push(Some(diff.after_cell_exclusion.len() as u64));
            avg_cost_with_disturbance.push(avg_cost(&diff.after_cell_exclusion));

            preferred_destination_without_disturbance
                .push(preferred_destination(&diff.before_cell_exclusion));
            preferred_destination_with_disturbance
                .push(preferred_destination(&diff.after_cell_exclusion));
        }

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                cell_h3indexes.into_arc(),
                preferred_destination_without_disturbance.into_arc(),
                preferred_destination_with_disturbance.into_arc(),
                population_at_origin.into_arc(),
                num_reached_without_disturbance.into_arc(),
                num_reached_with_disturbance.into_arc(),
                avg_cost_without_disturbance.into_arc(),
                avg_cost_with_disturbance.into_arc(),
            ],
        )?;
        batches.push(batch);
    }
    Ok(batches)
}

pub fn disturbance_statistics_status(
    output: &DspOutput,
) -> std::result::Result<Vec<RecordBatch>, Status> {
    let rbs = disturbance_statistics(output).map_err(|e| {
        log::error!("calculating population movement stats failed: {:?}", e);
        Status::internal("calculating population movement stats failed")
    })?;
    Ok(rbs)
}

pub fn build_routes_response(
    diff: &ExclusionDiff<Path<Weight>>,
) -> Result<DifferentialShortestPathRoutes, Status> {
    let response = DifferentialShortestPathRoutes {
        routes_without_disturbance: diff
            .before_cell_exclusion
            .iter()
            .map(|path| RouteWkb::from_path(path))
            .collect::<Result<_, _>>()?,
        routes_with_disturbance: diff
            .after_cell_exclusion
            .iter()
            .map(|path| RouteWkb::from_path(path))
            .collect::<Result<_, _>>()?,
    };
    Ok(response)
}
