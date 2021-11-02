use std::collections::HashMap;

use eyre::Result;
use serde::Deserialize;

use h3io::s3::{S3Config, S3H3Dataset};

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
    pub file_h3_resolution: u8,
    pub h3index_column_name: Option<String>,
}

impl S3H3Dataset for GenericDataset {
    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn key_pattern(&self) -> String {
        self.key_pattern.clone()
    }

    fn file_h3_resolution(&self) -> u8 {
        self.file_h3_resolution
    }

    fn h3index_column(&self) -> String {
        self.h3index_column_name
            .clone()
            .unwrap_or_else(|| "h3index".to_string())
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub s3: S3Config,
    pub graph_store: GraphStoreConfig,
    pub output: OutputConfig,
    pub datasets: HashMap<String, GenericDataset>,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        for dataset in self.datasets.values() {
            dataset.validate()?;
        }
        Ok(())
    }
}
