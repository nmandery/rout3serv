use std::collections::HashMap;

use crate::ui::ViewerStyleConfig;
use eyre::Result;
use s3io::s3::{S3Config, S3H3Dataset};
use serde::{Deserialize, Serialize};

use crate::util::Validate;

#[derive(Deserialize, Clone)]
pub struct TileDataset {
    pub key_pattern: String,
    pub bucket: String,

    /// maps data resolutions to the file h3 resolutions
    pub resolutions: HashMap<u8, u8>,

    pub h3index_column_name: Option<String>,

    /// optional styling for the tileset in the integrated viewer
    pub style: Option<ViewerStyleConfig>,
}

#[derive(Deserialize, Serialize, Clone)]
pub enum UiBaseLayer {
    #[serde(rename = "builtin")]
    Builtin,

    #[serde(rename = "eoc-baseoverlay")]
    EocBaseOverlay,
}

impl Default for UiBaseLayer {
    fn default() -> Self {
        Self::Builtin
    }
}

#[derive(Deserialize, Clone)]
pub struct UiConfig {
    #[serde(default)]
    pub enable_ui: bool,

    #[serde(default)]
    pub base_layer: UiBaseLayer,
}

#[allow(clippy::derivable_impls)]
impl Default for UiConfig {
    fn default() -> Self {
        Self {
            enable_ui: false,
            base_layer: UiBaseLayer::default(),
        }
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub cache_capacity: Option<usize>,
    pub s3: S3Config,
    pub datasets: HashMap<String, TileDataset>,

    #[serde(default)]
    pub ui: UiConfig,

    /// value for the cache-control header
    /// Defaults to `no-cache` when not set.
    pub cache_control: Option<String>,
}

impl Validate for ServerConfig {
    fn validate(&self) -> Result<()> {
        for dataset in self.datasets.values() {
            dataset.validate()?;
        }
        Ok(())
    }
}

impl S3H3Dataset for TileDataset {
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
