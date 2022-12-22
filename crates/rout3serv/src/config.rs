use std::collections::HashMap;
use std::ops::Deref;

use serde::Deserialize;
use tonic::Status;

use crate::io::dataframe::DataframeDataset;
use crate::io::objectstore::ObjectStoreConfig;

fn default_graphs_prefix() -> String {
    "graphs/".to_string()
}

#[derive(Deserialize, Clone)]
pub struct GraphsConfig {
    #[serde(default = "default_graphs_prefix")]
    pub prefix: String,

    /// capacity for the internal LRU cache
    pub cache_size: Option<usize>,
}

fn default_outputs_prefix() -> String {
    "outputs/".to_string()
}

#[derive(Deserialize, Clone)]
pub struct OutputsConfig {
    #[serde(default = "default_outputs_prefix")]
    pub prefix: String,
}

#[derive(Deserialize, Clone, Default, Copy)]
#[serde(try_from = "f32")]
pub struct NonZeroPositiveFactor(f32);

impl TryFrom<f32> for NonZeroPositiveFactor {
    type Error = anyhow::Error;

    fn try_from(value: f32) -> std::result::Result<Self, Self::Error> {
        if !value.is_normal() || value <= 0.0 {
            Err(Self::Error::msg("value must be > 0.0"))
        } else {
            Ok(Self(value))
        }
    }
}

impl Deref for NonZeroPositiveFactor {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct RoutingMode {
    /// factor to which degree the type of edge is included in the cost calculation
    ///
    /// Default is None, which means only the travel_duration is taken into account
    pub edge_preference_factor: Option<NonZeroPositiveFactor>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub objectstore: ObjectStoreConfig,
    pub graphs: GraphsConfig,
    pub outputs: OutputsConfig,
    pub datasets: HashMap<String, DataframeDataset>,

    #[serde(default)]
    pub routing_modes: HashMap<String, RoutingMode>,
}

impl ServerConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        for dataset in self.datasets.values() {
            dataset.validate()?;
        }
        Ok(())
    }

    pub fn get_routing_mode(&self, routing_mode_name: &str) -> Result<RoutingMode, Status> {
        if routing_mode_name.is_empty() {
            return Ok(RoutingMode::default());
        }
        self.routing_modes
            .get(routing_mode_name)
            .cloned()
            .ok_or_else(|| Status::invalid_argument("unknown routing_mode"))
    }
}
