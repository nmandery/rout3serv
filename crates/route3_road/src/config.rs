use std::collections::HashMap;
use std::ops::Deref;

use eyre::{Report, Result};
use serde::Deserialize;
use tonic::Status;

use s3io::s3::{S3Config, S3H3Dataset};

#[derive(Deserialize, Clone)]
pub struct GraphStoreConfig {
    pub prefix: String,
    pub bucket: String,

    /// capacity for the internal LRU cache
    pub cache_size: Option<usize>,
}

#[derive(Deserialize, Clone)]
pub struct OutputConfig {
    pub key_prefix: Option<String>,
    pub bucket: String,
}

#[derive(Deserialize)]
pub struct GenericDataset {
    pub key_pattern: String,
    pub bucket: String,
    /// maps data resolutions to the file h3 resolutions
    pub resolutions: HashMap<u8, u8>,

    pub h3index_column_name: Option<String>,
}

impl S3H3Dataset for GenericDataset {
    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn key_pattern(&self) -> String {
        self.key_pattern.clone()
    }

    fn h3index_column(&self) -> String {
        self.h3index_column_name
            .clone()
            .unwrap_or_else(|| "h3index".to_string())
    }

    fn file_h3_resolution(&self, data_h3_resolution: u8) -> Option<u8> {
        self.resolutions.get(&data_h3_resolution).copied()
    }
}

#[derive(Deserialize, Clone, Default, Copy)]
#[serde(try_from = "f32")]
pub struct NonZeroPositiveFactor(f32);

impl TryFrom<f32> for NonZeroPositiveFactor {
    type Error = Report;

    fn try_from(value: f32) -> std::result::Result<Self, Self::Error> {
        if !value.is_normal() || value <= 0.0 {
            Err(Report::msg("value must be > 0.0"))
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
    pub s3: S3Config,
    pub graph_store: GraphStoreConfig,
    pub output: OutputConfig,
    pub datasets: HashMap<String, GenericDataset>,

    #[serde(default)]
    pub routing_modes: HashMap<String, RoutingMode>,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
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
