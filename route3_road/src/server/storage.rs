use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::io::Cursor;
use std::sync::Arc;

use h3ron::collections::H3CellSet;
use h3ron::io::{deserialize_from, serialize_into};
use h3ron::iter::change_cell_resolution;
use h3ron::H3Cell;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use polars_core::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::task::block_in_place;
use tonic::Status;

use crate::config::{GenericDataset, ServerConfig};
use crate::io::dataframe::H3DataFrame;
use crate::io::graph_store::{GraphCacheKey, GraphStore};
use crate::io::s3::{FoundOption, S3Client, S3RecordBatchLoader};
use crate::server::api::generated::{CellSelection, GraphHandle};
use crate::server::util::StrId;

/// storage backend to use in the server.
///
/// most member functions directly return [`Status`] errors the be passed
/// to tonic.
pub struct S3Storage<W: Send + Sync> {
    s3_client: Arc<S3Client>,
    pub graph_store: GraphStore<W>,
    config: Arc<ServerConfig>,
    recordbatch_loader: S3RecordBatchLoader,
}

impl<W: Send + Sync> S3Storage<W>
where
    W: Serialize + DeserializeOwned,
{
    pub fn from_config(config: Arc<ServerConfig>) -> eyre::Result<Self> {
        let s3_client = Arc::new(S3Client::from_config(&config.s3)?);
        let graph_store = GraphStore::new(s3_client.clone(), config.graph_store.clone());
        let recordbatch_loader = S3RecordBatchLoader::new(s3_client.clone());
        Ok(Self {
            s3_client,
            graph_store,
            config,
            recordbatch_loader,
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
        let serialized = block_in_place(move || {
            let mut serialized: Vec<u8> = Default::default();
            match serialize_into(&mut serialized, output, true) {
                Ok(_) => Ok(serialized),
                Err(e) => {
                    log::error!("serializing output failed: {:?}", e);
                    Err(Status::internal("serializing output failed"))
                }
            }
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
                let output: O = block_in_place(move || deserialize_from(Cursor::new(&bytes)))
                    .map_err(|e| {
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

    pub fn get_dataset_config<B>(
        &self,
        dataset_name: B,
    ) -> std::result::Result<&GenericDataset, Status>
    where
        B: Borrow<String>,
    {
        let ds_name = dataset_name.borrow().trim().to_string();
        if ds_name.is_empty() {
            log::warn!("empty dataset name given");
            return Err(Status::invalid_argument("empty dataset name given"));
        }
        self.config.datasets.get(&ds_name).ok_or_else(|| {
            log::warn!("unknown dataset requested: {}", ds_name);
            Status::invalid_argument(format!("unknown dataset: {}", ds_name))
        })
    }

    pub async fn load_dataset_dataframe(
        &self,
        dataset_config: &GenericDataset,
        cells: &[H3Cell],
        data_h3_resolution: u8,
    ) -> std::result::Result<H3DataFrame, Status> {
        let mut h3dataframe = self
            .recordbatch_loader
            .load_h3_dataset_dataframe(dataset_config, cells, data_h3_resolution)
            .await
            .map_err(|e| {
                log::error!("loading from s3 failed: {:?}", e);
                Status::internal("dataset is inaccessible")
            })?;
        if !h3dataframe.dataframe.is_empty() {
            h3dataframe.dataframe.rechunk();
        }
        Ok(h3dataframe)
    }

    pub fn list_datasets(&self) -> Vec<String> {
        self.config.datasets.keys().cloned().collect()
    }

    /// fetch all contents required for the `cell_selection`.
    ///
    /// Input cells will get:
    /// * transformed to `h3_resolution`
    /// * filtered by the dataset given using the `dataset_name` in the `CellSelection`
    /// * invalid cells will get removed
    ///
    /// In case the `dataset_name` is set, the `DataFrame` for this dataset will
    /// be returned as well.
    pub async fn load_cell_selection(
        &self,
        cell_selection: &CellSelection,
        h3_resolution: u8,
    ) -> std::result::Result<(Vec<H3Cell>, Option<H3DataFrame>), Status> {
        // build a complete list of the requested h3cells transformed to the
        // correct resolution
        let mut cells: Vec<_> = change_cell_resolution(
            cell_selection
                .cells
                .iter()
                .filter_map(|v| match H3Cell::try_from(*v) {
                    Ok(cell) => Some(cell),
                    Err(_) => {
                        log::warn!("invalid h3 index {} ignored", v);
                        None
                    }
                }),
            h3_resolution,
        )
        .collect();
        cells.sort_unstable();
        cells.dedup();

        if cells.is_empty() || cell_selection.dataset_name.is_empty() {
            Ok((cells, None))
        } else {
            let df = self
                .load_dataset_dataframe(
                    self.get_dataset_config(&cell_selection.dataset_name)?,
                    &cells,
                    h3_resolution,
                )
                .await?;

            let reduced_cells =
                filter_cells_by_dataframe_contents(&df.dataframe, cells, &df.h3index_column_name)
                    .map_err(|e| {
                    log::error!("reducing input cell selection failed: {:?}", e);
                    Status::internal("reducing input cell selection failed")
                })?;
            Ok((reduced_cells, Some(df)))
        }
    }
}

fn filter_cells_by_dataframe_contents(
    df: &DataFrame,
    mut input_cells: Vec<H3Cell>,
    h3index_column_name: &str,
) -> eyre::Result<Vec<H3Cell>> {
    if df.is_empty() {
        return Ok(Default::default());
    }

    let df_cells_lookup: H3CellSet = df
        .column(h3index_column_name)?
        .u64()?
        .into_iter()
        .filter_map(|v| v.map(|i| H3Cell::try_from(i).ok()).flatten())
        .collect();

    Ok(input_cells
        .drain(..)
        .filter(|cell| df_cells_lookup.contains(cell))
        .collect())
}
