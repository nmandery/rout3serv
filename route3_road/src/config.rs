use serde::Deserialize;

use crate::io::s3::{S3Config, S3H3Dataset};

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

#[derive(Deserialize, Clone)]
pub struct PopulationDatasetConfig {
    pub key_pattern: String,
    pub bucket: String,
    pub file_h3_resolution: u8,
    pub h3index_column_name: Option<String>,
    pub population_count_column_name: Option<String>,
}

impl PopulationDatasetConfig {
    pub fn get_h3index_column_name(&self) -> String {
        self.h3index_column_name
            .clone()
            .unwrap_or_else(|| "h3index".to_string())
    }

    pub fn get_population_count_column_name(&self) -> String {
        self.population_count_column_name
            .clone()
            .unwrap_or_else(|| "population".to_string())
    }
}

impl S3H3Dataset for PopulationDatasetConfig {
    fn bucket_name(&self) -> String {
        self.bucket.clone()
    }

    fn key_pattern(&self) -> String {
        self.key_pattern.clone()
    }

    fn file_h3_resolution(&self) -> u8 {
        self.file_h3_resolution
    }
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bind_to: String,
    pub s3: S3Config,
    pub graph_store: GraphStoreConfig,
    pub population_dataset: PopulationDatasetConfig,
    pub output: OutputConfig,
}
