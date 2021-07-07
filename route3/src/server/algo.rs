use std::sync::Arc;

use arrow::record_batch::RecordBatch;
use eyre::Result;
use serde::{Deserialize, Serialize};

use route3_core::h3ron::{H3Cell, Index};
use route3_core::routing::{ManyToManyOptions, Route, RoutingContext};
use route3_core::H3CellMap;

use crate::constants::Weight;
use crate::server::api::DisturbanceOfPopulationMovementInput;
use arrow::array::{Float64Array, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};

#[derive(Serialize, Deserialize)]
pub enum StorableOutput {
    DisturbanceOfPopulationMovement(DisturbanceOfPopulationMovementOutput),
}

impl StorableOutput {
    pub fn id(&self) -> &str {
        match self {
            StorableOutput::DisturbanceOfPopulationMovement(dopm) => dopm.id.as_ref(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DisturbanceOfPopulationMovementOutput {
    pub id: String,
    pub input: DisturbanceOfPopulationMovementInput,

    pub population_within_disturbance: f64,
    pub population_at_origins: H3CellMap<f64>,

    /// keyed with the origin-cell
    pub routes_without_disturbance: H3CellMap<Vec<Route<Weight>>>,

    /// keyed with the origin-cell
    pub routes_with_disturbance: H3CellMap<Vec<Route<Weight>>>,
}

impl From<DisturbanceOfPopulationMovementOutput> for StorableOutput {
    fn from(inner: DisturbanceOfPopulationMovementOutput) -> Self {
        Self::DisturbanceOfPopulationMovement(inner)
    }
}

pub fn disturbance_of_population_movement(
    routing_context: Arc<RoutingContext<Weight>>,
    input: DisturbanceOfPopulationMovementInput,
    population: H3CellMap<f32>,
) -> Result<DisturbanceOfPopulationMovementOutput> {
    let population_within_disturbance = input
        .disturbance
        .iter()
        .filter_map(|cell| population.get(cell))
        .sum::<f32>() as f64;

    let mut population_at_origins: H3CellMap<f64> = H3CellMap::new();
    let mut origin_cells: Vec<H3Cell> = vec![];
    for cell in input.within_buffer.iter() {
        // exclude the cells of the disturbance itself as well as all origin cells without
        // any population from routing
        if let (Some(pop), false) = (population.get(cell), input.disturbance.contains(cell)) {
            population_at_origins.insert(*cell, *pop as f64);
            origin_cells.push(*cell);
        }
    }

    let routes_without_disturbance = routing_context.route_many_to_many(
        &origin_cells,
        &input.destinations,
        &ManyToManyOptions {
            num_destinations_to_reach: input.num_destinations_to_reach,
            ..Default::default()
        },
    )?;

    let routes_with_disturbance = routing_context.route_many_to_many(
        &origin_cells,
        &input.destinations,
        &ManyToManyOptions {
            num_destinations_to_reach: input.num_destinations_to_reach,
            exclude_cells: Some(input.disturbance.clone()),
        },
    )?;

    Ok(DisturbanceOfPopulationMovementOutput {
        id: uuid::Uuid::new_v4().to_string(),
        input,
        population_within_disturbance,
        population_at_origins,
        routes_without_disturbance,
        routes_with_disturbance,
    })
}

struct DOPMOWeights {
    pub with_disturbance: Vec<Weight>,
    pub without_disturbance: Vec<Weight>,
    /// preferred destination cell
    pub preferred_destination_with_disturbance: Option<u64>,
    pub preferred_destination_without_disturbance: Option<u64>,
}

impl Default for DOPMOWeights {
    fn default() -> Self {
        Self {
            with_disturbance: vec![],
            without_disturbance: vec![],
            preferred_destination_with_disturbance: None,
            preferred_destination_without_disturbance: None,
        }
    }
}

impl DisturbanceOfPopulationMovementOutput {
    /// build an arrow dataset with some basic stats for each of the origin cells
    pub fn stats_recordbatch(&self) -> Result<RecordBatch> {
        // TODO: this code is ugly - improve this
        let mut aggregated_weights: H3CellMap<DOPMOWeights> = H3CellMap::new();
        for (origin_cell, routes) in self.routes_without_disturbance.iter() {
            let entry = aggregated_weights.entry(*origin_cell).or_default();
            for route in routes.iter() {
                if !entry.without_disturbance.iter().any(|w| &route.cost < w) {
                    entry.preferred_destination_without_disturbance =
                        Some(route.destination_cell()?.h3index());
                }
                entry.without_disturbance.push(route.cost);
            }
        }
        for (origin_cell, routes) in self.routes_with_disturbance.iter() {
            let entry = aggregated_weights.entry(*origin_cell).or_default();
            for route in routes.iter() {
                if !entry.with_disturbance.iter().any(|w| &route.cost < w) {
                    entry.preferred_destination_with_disturbance =
                        Some(route.destination_cell()?.h3index());
                }
                entry.with_disturbance.push(route.cost);
            }
        }

        let mut cell_h3indexes = Vec::with_capacity(aggregated_weights.len());
        let mut population = Vec::with_capacity(aggregated_weights.len());
        let mut num_reached_without_disturbance = Vec::with_capacity(aggregated_weights.len());
        let mut num_reached_with_disturbance = Vec::with_capacity(aggregated_weights.len());
        let mut avg_cost_without_disturbance = Vec::with_capacity(aggregated_weights.len());
        let mut avg_cost_with_disturbance = Vec::with_capacity(aggregated_weights.len());
        let mut preferred_destination_without_disturbance =
            Vec::with_capacity(aggregated_weights.len());
        let mut preferred_destination_with_disturbance =
            Vec::with_capacity(aggregated_weights.len());
        for (cell, agg_weight) in aggregated_weights.drain() {
            cell_h3indexes.push(cell.h3index() as u64);
            population.push(self.population_at_origins.get(&cell).cloned());

            let num_reached_wo_d = agg_weight.without_disturbance.len() as u64;
            num_reached_without_disturbance.push(num_reached_wo_d);
            avg_cost_without_disturbance.push(if num_reached_wo_d > 0 {
                Some(
                    agg_weight
                        .without_disturbance
                        .iter()
                        .map(|weight| f64::from(*weight))
                        .sum::<f64>()
                        / num_reached_wo_d as f64,
                )
            } else {
                None
            });

            let num_reached_w_d = agg_weight.with_disturbance.len() as u64;
            num_reached_with_disturbance.push(num_reached_w_d);
            avg_cost_with_disturbance.push(if num_reached_w_d > 0 {
                Some(
                    agg_weight
                        .with_disturbance
                        .iter()
                        .map(|weight| f64::from(*weight))
                        .sum::<f64>()
                        / num_reached_w_d as f64,
                )
            } else {
                None
            });

            preferred_destination_without_disturbance
                .push(agg_weight.preferred_destination_without_disturbance);
            preferred_destination_with_disturbance
                .push(agg_weight.preferred_destination_with_disturbance);
        }

        let h3index_origin_array = UInt64Array::from(cell_h3indexes);
        let population_origin_array = Float64Array::from(population);
        let num_reached_without_disturbance_array =
            UInt64Array::from(num_reached_without_disturbance);
        let num_reached_with_disturbance_array = UInt64Array::from(num_reached_with_disturbance);
        let avg_cost_without_disturbance_array = Float64Array::from(avg_cost_without_disturbance);
        let avg_cost_with_disturbance_array = Float64Array::from(avg_cost_with_disturbance);
        let preferred_destination_without_disturbance_array =
            UInt64Array::from(preferred_destination_without_disturbance);
        let preferred_destination_with_disturbance_array =
            UInt64Array::from(preferred_destination_with_disturbance);

        let schema = Schema::new(vec![
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
            Field::new("population_origin", DataType::Float64, true), // TODO: should be nullable instead of NAN values
            Field::new("num_reached_without_disturbance", DataType::UInt64, false),
            Field::new("num_reached_with_disturbance", DataType::UInt64, false),
            Field::new("avg_cost_without_disturbance", DataType::Float64, true),
            Field::new("avg_cost_with_disturbance", DataType::Float64, true),
        ]);

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(h3index_origin_array),
                Arc::new(preferred_destination_without_disturbance_array),
                Arc::new(preferred_destination_with_disturbance_array),
                Arc::new(population_origin_array),
                Arc::new(num_reached_without_disturbance_array),
                Arc::new(num_reached_with_disturbance_array),
                Arc::new(avg_cost_without_disturbance_array),
                Arc::new(avg_cost_with_disturbance_array),
            ],
        )?;
        Ok(batch)
    }
}
