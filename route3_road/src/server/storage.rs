use std::convert::TryInto;
use std::io::Cursor;
use std::sync::Arc;

use h3ron::collections::H3CellMap;
use h3ron::io::{deserialize_from, serialize_into};
use h3ron::H3Cell;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tonic::Status;

use crate::config::ServerConfig;
use crate::io::graph_store::{GraphCacheKey, GraphStore};
use crate::io::s3::{FoundOption, S3Client, S3RecordBatchLoader};
use crate::server::api::generated::GraphHandle;
use crate::server::util::StrId;

/// storage backend to use in the server.
///
/// most member functions directly return [`Status`] errors the be passed
/// to tonic.
pub struct S3Storage<W: Send + Sync> {
    s3_client: Arc<S3Client>,
    pub graph_store: GraphStore<W>,
    config: Arc<ServerConfig>,
}

impl<W: Send + Sync> S3Storage<W>
where
    W: Serialize + DeserializeOwned,
{
    pub fn from_config(config: Arc<ServerConfig>) -> eyre::Result<Self> {
        let s3_client = Arc::new(S3Client::from_config(&config.s3)?);
        let graph_store = GraphStore::new(s3_client.clone(), config.graph_store.clone());
        Ok(Self {
            s3_client,
            graph_store,
            config,
        })
    }

    fn output_s3_key<I: AsRef<str>>(&self, id: I) -> String {
        format!(
            "{}.bincode.lz",
            self.config
                .output
                .key_prefix
                .as_ref()
                .map(|prefix| format!("{}{}", prefix, id.as_ref()))
                .unwrap_or_else(|| id.as_ref().to_string())
        )
    }

    pub async fn store_output<O: Serialize + StrId>(
        &self,
        output: &O,
    ) -> std::result::Result<(), Status> {
        let mut serialized: Vec<u8> = Default::default();
        serialize_into(&mut serialized, output, true).map_err(|e| {
            log::error!("serializing output failed: {:?}", e);
            Status::internal("serializing output failed")
        })?;
        self.s3_client
            .put_object_bytes(
                self.config.output.bucket.clone(),
                self.output_s3_key(output.id()),
                serialized,
            )
            .await
            .map_err(|e| {
                log::error!("storing output failed: {:?}", e);
                Status::internal("storing output failed")
            })?;
        Ok(())
    }

    pub async fn retrieve_output<I: AsRef<str>, O: DeserializeOwned>(
        &self,
        id: I,
    ) -> std::result::Result<FoundOption<O>, Status> {
        let key = self.output_s3_key(id);
        let found_option = match self
            .s3_client
            .get_object_bytes(self.config.output.bucket.clone(), key.clone())
            .await
            .map_err(|e| {
                log::error!("retrieving output with key = {} failed: {:?}", key, e);
                Status::internal(format!("retrieving output with key = {} failed", key))
            })? {
            FoundOption::Found(bytes) => {
                let output: O = deserialize_from(Cursor::new(&bytes)).map_err(|e| {
                    log::error!("deserializing output with key = {} failed: {:?}", key, e);
                    Status::internal(format!("deserializing output with key = {} failed", key))
                })?;
                FoundOption::Found(output)
            }
            FoundOption::NotFound => FoundOption::NotFound,
        };
        Ok(found_option)
    }

    pub async fn load_graph_cache_keys(&self) -> std::result::Result<Vec<GraphCacheKey>, Status> {
        let gcks = self.graph_store.list().await.map_err(|e| {
            log::error!("loading graph list failed: {:?}", e);
            Status::internal("loading graph list failed")
        })?;
        Ok(gcks)
    }

    pub async fn load_graph(
        &self,
        graph_cache_key: &GraphCacheKey,
    ) -> std::result::Result<Arc<PreparedH3EdgeGraph<W>>, Status> {
        match self.graph_store.load(graph_cache_key).await.map_err(|e| {
            log::error!("could not load graph: {:?}", e);
            Status::internal("could not load graph")
        })? {
            None => Err(Status::not_found("graph not found")),
            Some(graph) => Ok(graph),
        }
    }

    pub async fn load_graph_from_option(
        &self,
        graph_handle: &Option<GraphHandle>,
    ) -> std::result::Result<(Arc<PreparedH3EdgeGraph<W>>, GraphCacheKey), Status> {
        if let Some(gh) = graph_handle {
            let gck: GraphCacheKey = gh.try_into().map_err(|e| {
                log::warn!("invalid graph handle: {:?}", e);
                Status::invalid_argument("invalid graph handle")
            })?;
            self.load_graph(&gck).await.map(|graph| (graph, gck))
        } else {
            Err(Status::invalid_argument("graph handle not set"))
        }
    }

    pub async fn load_population(
        &self,
        h3_resolution: u8,
        cells: &[H3Cell],
    ) -> std::result::Result<H3CellMap<f32>, Status> {
        let loader = S3RecordBatchLoader::new(self.s3_client.clone());
        let population_cellmap = loader
            .load_h3_dataset_cellmap(
                &self.config.population_dataset,
                cells,
                h3_resolution,
                &self.config.population_dataset.get_h3index_column_name(),
                &self
                    .config
                    .population_dataset
                    .get_population_count_column_name(),
            )
            .await
            .map_err(|e| {
                log::error!("loading population from s3 failed: {:?}", e);
                Status::internal("population data inaccessible")
            })?;
        Ok(population_cellmap)
    }
}
