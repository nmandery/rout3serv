use std::sync::Arc;

use eyre::Result;
use serde::{Deserialize, Serialize};

use route3_core::h3ron::H3Cell;
use route3_core::routing::{ManyToManyOptions, Route, RoutingContext};
use route3_core::H3CellMap;

use crate::constants::WeightType;
use crate::server::api::DisturbanceOfPopulationMovementInput;

#[derive(Serialize, Deserialize)]
enum StorableOutput {
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

    pub routes_without_disturbance: Vec<Route<WeightType>>,
    pub routes_with_disturbance: Vec<Route<WeightType>>,
}

impl From<DisturbanceOfPopulationMovementOutput> for StorableOutput {
    fn from(inner: DisturbanceOfPopulationMovementOutput) -> Self {
        Self::DisturbanceOfPopulationMovement(inner)
    }
}

pub fn disturbance_of_population_movement(
    routing_context: Arc<RoutingContext<WeightType>>,
    input: DisturbanceOfPopulationMovementInput,
    population: H3CellMap<f32>,
) -> Result<DisturbanceOfPopulationMovementOutput> {
    let population_within_disturbance = input
        .disturbance
        .iter()
        .filter_map(|cell| population.get(cell))
        .sum::<f32>() as f64;

    let routing_start_cells: Vec<H3Cell> = input
        .within_buffer
        .iter()
        .filter(|cell| !input.disturbance.contains(cell))
        .cloned()
        .collect();

    let options_without_disturbance = ManyToManyOptions {
        num_destinations_to_reach: input.num_destinations_to_reach,
        ..Default::default()
    };
    let routes_without_disturbance = routing_context.route_many_to_many(
        &routing_start_cells,
        &input.destinations,
        &options_without_disturbance,
    )?;

    let options_with_disturbance = ManyToManyOptions {
        num_destinations_to_reach: input.num_destinations_to_reach,
        exclude_cells: Some(input.disturbance.clone()),
    };
    let routes_with_disturbance = routing_context.route_many_to_many(
        &routing_start_cells,
        &input.destinations,
        &options_with_disturbance,
    )?;

    Ok(DisturbanceOfPopulationMovementOutput {
        id: uuid::Uuid::new_v4().to_string(),
        input,
        population_within_disturbance,
        routes_without_disturbance,
        routes_with_disturbance,
    })
}
