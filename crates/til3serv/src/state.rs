use crate::config::{ServerConfig, TileDataset};
use crate::tile::CellBuilder;
use s3io::s3::{S3Client, S3RecordBatchLoader};
use std::collections::HashMap;
use std::sync::Arc;

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
    pub loader: S3RecordBatchLoader,
}

impl TryFrom<ServerConfig> for Registry {
    type Error = eyre::Error;

    fn try_from(mut server_config: ServerConfig) -> Result<Self, Self::Error> {
        let s3_client: Arc<S3Client> = Arc::new(S3Client::from_config(&server_config.s3)?);
        let reg = Self {
            datasets: server_config
                .datasets
                .drain()
                .map(|(name, tds)| (name, tds.into()))
                .collect(),
            loader: S3RecordBatchLoader::new(
                s3_client,
                server_config.cache_capacity.unwrap_or(120),
            ),
        };
        Ok(reg)
    }
}
