use std::sync::Arc;

use arrow2::array::{Float64Vec, UInt64Vec};
use arrow2::datatypes::{DataType, Field, Schema};
use arrow2::record_batch::RecordBatch;
use eyre::Result;
use h3ron::collections::{H3CellMap, H3CellSet};
use h3ron::{H3Cell, Index};
use h3ron_graph::algorithm::differential_shortest_path::{
    differential_shortest_path, DifferentialShortestPath,
};
use h3ron_graph::algorithm::path::Path;
use h3ron_graph::algorithm::shortest_path::ShortestPathOptions;
use h3ron_graph::routing::RoutingH3EdgeGraph;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tonic::Status;

use crate::server::util::StrId;
use crate::types::Weight;

#[derive(Serialize, Deserialize)]
pub struct Input {
    /// the cells within the disturbance
    pub disturbance: H3CellSet,

    /// the cells of the disturbance and within the surrounding buffer
    pub within_buffer: Vec<H3Cell>,

    /// the destination cells to route to
    pub destinations: Vec<H3Cell>,

    pub num_destinations_to_reach: Option<usize>,
    pub num_gap_cells_to_graph: u32,
    pub downsampled_prerouting: bool,
    pub store_output: bool,
}

impl ShortestPathOptions<Weight> for Input {
    fn exclude_cells(&self) -> Option<H3CellSet> {
        Some(self.disturbance.clone())
    }

    fn num_gap_cells_to_graph(&self) -> u32 {
        self.num_gap_cells_to_graph
    }

    fn num_destinations_to_reach(&self) -> Option<usize> {
        self.num_destinations_to_reach
    }
}

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub dopm_id: String,
    pub population_within_disturbance: f64,
    pub population_at_origins: H3CellMap<f64>,
    pub differential_shortest_paths: Vec<DifferentialShortestPath<Path<Weight>>>,
}

impl StrId for Output {
    fn id(&self) -> &str {
        self.dopm_id.as_ref()
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
    routing_graph: Arc<RoutingH3EdgeGraph<Weight>>,
    input: Input,
    population: H3CellMap<f32>,
    downsampled_routing_graph: Option<Arc<RoutingH3EdgeGraph<Weight>>>,
) -> Result<Output> {
    let population_within_disturbance = input
        .disturbance
        .iter()
        .filter_map(|cell| population.get(cell))
        .sum::<f32>() as f64;

    let mut population_at_origins: H3CellMap<f64> = H3CellMap::default();

    let origin_cells: Vec<H3Cell> = {
        let mut origin_cells = Vec::with_capacity(input.within_buffer.len());
        for cell in input.within_buffer.iter() {
            // exclude the cells of the disturbance itself as well as all origin cells without
            // any population from routing
            if let (Some(pop), false) = (population.get(cell), input.disturbance.contains(cell)) {
                population_at_origins.insert(*cell, *pop as f64);
                origin_cells.push(*cell);
            }
        }
        origin_cells
    };

    let differential_shortest_paths = differential_shortest_path(
        routing_graph,
        &origin_cells,
        &input.destinations,
        downsampled_routing_graph,
        &input,
    )?;

    Ok(Output {
        dopm_id: uuid::Uuid::new_v4().to_string(),
        population_within_disturbance,
        population_at_origins,
        differential_shortest_paths,
    })
}

/// build an arrow dataset with some basic stats for each of the origin cells
pub fn disturbance_statistics(output: &Output) -> Result<Vec<RecordBatch>> {
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
        Field::new("population_origin", DataType::Float64, true),
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
        let mut population_at_origin = Float64Vec::with_capacity(chunk_size);
        let mut num_reached_without_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut num_reached_with_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut avg_cost_without_disturbance = Float64Vec::with_capacity(chunk_size);
        let mut avg_cost_with_disturbance = Float64Vec::with_capacity(chunk_size);
        let mut preferred_destination_without_disturbance = UInt64Vec::with_capacity(chunk_size);
        let mut preferred_destination_with_disturbance = UInt64Vec::with_capacity(chunk_size);
        for differential_shortest_path in chunk {
            cell_h3indexes.push(Some(differential_shortest_path.origin_cell.h3index() as u64));
            population_at_origin.push(
                output
                    .population_at_origins
                    .get(&differential_shortest_path.origin_cell)
                    .cloned(),
            );

            num_reached_without_disturbance.push(Some(
                differential_shortest_path.without_disturbance.len() as u64,
            ));
            avg_cost_without_disturbance
                .push(avg_cost(&differential_shortest_path.without_disturbance));

            num_reached_with_disturbance.push(Some(
                differential_shortest_path.with_disturbance.len() as u64,
            ));
            avg_cost_with_disturbance.push(avg_cost(&differential_shortest_path.with_disturbance));

            preferred_destination_without_disturbance.push(preferred_destination(
                &differential_shortest_path.without_disturbance,
            ));
            preferred_destination_with_disturbance.push(preferred_destination(
                &differential_shortest_path.with_disturbance,
            ));
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
    output: &Output,
) -> std::result::Result<Vec<RecordBatch>, Status> {
    let rbs = disturbance_statistics(output).map_err(|e| {
        log::error!("calculating population movement stats failed: {:?}", e);
        Status::internal("calculating population movement stats failed")
    })?;
    Ok(rbs)
}
