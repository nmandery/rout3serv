use std::collections::HashMap;
use std::sync::Arc;

use axum::http::HeaderValue;

use s3io::s3::{S3ArrowLoader, S3Client};
use slippymap_h3_tiles::CellBuilder;

use crate::config::{ServerConfig, TileDataset, UiConfig};

pub struct WrappedTileDataset {
    pub tile_dataset: TileDataset,
    pub cell_builder: CellBuilder,
}

impl From<TileDataset> for WrappedTileDataset {
    fn from(tds: TileDataset) -> Self {
        let cell_builder = CellBuilder::new(tds.resolutions.keys());
        Self {
            tile_dataset: tds,
            cell_builder,
        }
    }
}

pub struct Registry {
    pub datasets: HashMap<String, WrappedTileDataset>,
    pub loader: S3ArrowLoader,

    /// value for the cache-control header
    pub cache_control: HeaderValue,

    pub ui: UiConfig,
}

impl TryFrom<ServerConfig> for Registry {
    type Error = anyhow::Error;

    fn try_from(mut server_config: ServerConfig) -> Result<Self, Self::Error> {
        let s3_client: Arc<S3Client> = Arc::new(S3Client::from_config(&server_config.s3)?);

        let datasets = server_config
            .datasets
            .drain()
            .map(|(name, tds)| (name, tds.into()))
            .collect();

        let cache_control = server_config
            .cache_control
            .map(|cc| HeaderValue::from_str(cc.as_str()))
            .unwrap_or_else(|| Ok(HeaderValue::from_static("no-cache")))?;

        let reg = Self {
            datasets,
            loader: S3ArrowLoader::new(s3_client, server_config.cache_capacity.unwrap_or(120)),
            cache_control,
            ui: server_config.ui.clone(),
        };
        Ok(reg)
    }
}
