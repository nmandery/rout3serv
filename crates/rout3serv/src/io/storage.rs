use std::io::Cursor;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use bytesize::ByteSize;
use futures::future::try_join_all;
use futures::TryStreamExt;
use h3ron::collections::HashSet;
use h3ron::iter::change_resolution;
use h3ron::H3Cell;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use h3ron_polars::frame::H3DataFrame;
use object_store::path::Path;
use once_cell::sync::Lazy;
use polars::prelude::DataFrame;
use polars_core::utils::concat_df;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::task;
use tokio::task::block_in_place;
use tracing::{debug, error, info};

use crate::config::ServerConfig;
use crate::io::dataframe::{DataframeDataset, FromDataFrame};
use crate::io::memory_cache::{CacheFetcher, FetchError, MemoryCache};
use crate::io::objectstore::ObjectStore;
use crate::io::parquet::ReadParquet;
use crate::io::serde_util::{deserialize_from_byte_slice, serialize_into};
use crate::io::{Error, GraphKey};

pub struct Storage<W: Sync + DeserializeOwned> {
    objectstore: Arc<ObjectStore>,
    graphs: MemoryCache<GraphFetcher<W>>,
}

impl<W: Sync + DeserializeOwned> Storage<W> {
    pub fn from_config(config: &ServerConfig) -> Result<Self, Error> {
        let objectstore = Arc::new(ObjectStore::try_from(config.objectstore.clone())?);
        let graphs = MemoryCache::new(
            config.graphs.cache_size.unwrap_or(10),
            GraphFetcher {
                prefix: config.graphs.prefix.clone(),
                phantom: Default::default(),
            },
        );

        Ok(Self {
            objectstore,
            graphs,
        })
    }

    pub async fn store<T>(&self, path: &Path, data: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let serialized: Bytes = block_in_place(move || {
            let mut serialized: Vec<u8> = Vec::with_capacity(50_000);
            serialize_into(&mut serialized, data, true).map(|_| serialized.into())
        })?;

        self.objectstore
            .0
            .put(path, serialized)
            .await
            .map_err(Error::from)
    }

    pub async fn retrieve<T>(&self, path: &Path) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        fetch(&self.objectstore, path, |bytes| {
            deserialize_from_byte_slice(bytes.as_ref())
        })
        .await
    }

    pub async fn retrieve_graph(
        &self,
        graph_key: GraphKey,
    ) -> Result<Arc<PreparedH3EdgeGraph<W>>, FetchError<Error>> {
        self.graphs
            .get_from(self.objectstore.clone(), graph_key)
            .await
    }

    pub async fn list_graphs(&self) -> Result<Vec<GraphKey>, Error> {
        self.graphs.inner().list(self.objectstore.clone()).await
    }

    pub async fn retrieve_dataframe(
        &self,
        dataset: &DataframeDataset,
        cells: &[H3Cell],
        data_h3_resolution: u8,
    ) -> Result<Option<H3DataFrame<H3Cell>>, Error> {
        if cells.is_empty() {
            return Ok(Default::default());
        }
        let fileformat = dataset.fileformat()?;
        let file_cells = change_resolution(
            cells.iter(),
            dataset.file_h3_resolution(data_h3_resolution)?,
        )
        .collect::<Result<HashSet<_>, _>>()?;

        let mut paths = file_cells
            .iter()
            .map(|cell| build_dataset_path(dataset, cell, data_h3_resolution))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort_unstable(); // remove duplicates when the keys are not grouped using a file resolution
        paths.dedup();

        let task_results = try_join_all(paths.into_iter().map(|path| {
            let objectstore = self.objectstore.clone();
            task::spawn(async move {
                debug!("Loading dataset file {}", path);
                objectstore
                    .get(&path)
                    .await
                    .map_err(|e| (e, path.clone()))
                    .map(|gr| (gr, path))
            })
        }))
        .await?;

        let mut dataframes = Vec::with_capacity(file_cells.len());
        for task_result in task_results.into_iter() {
            match task_result {
                Ok((getresult, path)) => {
                    match getresult.bytes().await {
                        Ok(bytes) => {
                            dataframes
                                .push(block_in_place(|| fileformat.dataframe_from_slice(&bytes))?);
                        }
                        Err(object_store::Error::NotFound { .. }) => {
                            // missing files are to be expected with sparse datasets
                            debug!("Dataset does not contain file {}", path);
                        }
                        Err(e) => {
                            error!("Dataset file {} could not be loaded: {:?}", path, e);
                            return Err(e.into());
                        }
                    }
                }
                Err((e, path)) => {
                    error!("Dataset file {} could not be requested: {:?}", path, e);
                    return Err(e.into());
                }
            }
        }
        let df = match dataframes.len() {
            0 => DataFrame::default(),
            1 => dataframes.pop().unwrap(),
            _ => {
                debug!("concatenating dataframe from {} parts", dataframes.len());
                block_in_place(|| concat_df(dataframes.iter()))?
            }
        };
        Ok(Some(H3DataFrame::from_dataframe(
            df,
            dataset.h3index_column_name.as_str(),
        )?))
    }
}

pub struct GraphFetcher<W> {
    prefix: String,
    phantom: PhantomData<W>,
}

impl<W> GraphFetcher<W> {
    pub fn prefix(&self) -> String {
        if self.prefix.is_empty() {
            "".to_string()
        } else {
            format!("{}/", self.prefix)
        }
    }

    pub async fn list(&self, objectstore: Arc<ObjectStore>) -> Result<Vec<GraphKey>, Error> {
        let p = self.prefix();
        let prefix_len = p.len();
        let path: Path = p.into();

        Ok(objectstore
            .list(Some(&path))
            .await?
            .try_filter_map(|object_meta| async move {
                Ok(GraphKey::from_str(
                    &object_meta.location.as_ref()[prefix_len.saturating_sub(1)..],
                )
                .ok())
            })
            .try_collect()
            .await?)
    }
}

#[async_trait::async_trait]
impl<W> CacheFetcher for GraphFetcher<W>
where
    W: Sync,
    PreparedH3EdgeGraph<W>: ReadParquet + FromDataFrame,
{
    type Key = GraphKey;
    type Value = PreparedH3EdgeGraph<W>;
    type Error = Error;

    async fn fetch_from(
        &self,
        objectstore: Arc<ObjectStore>,
        key: Self::Key,
    ) -> Result<Self::Value, Self::Error> {
        let path: Path = format!("{}{}", self.prefix(), key.to_string()).into();
        fetch(objectstore.as_ref(), &path, |bytes| {
            let cur = Cursor::new(bytes.as_ref());
            PreparedH3EdgeGraph::<W>::read_parquet(cur)
        })
        .await
    }
}

async fn fetch<T, F>(objectstore: &ObjectStore, path: &Path, f: F) -> Result<T, Error>
where
    F: FnOnce(Bytes) -> Result<T, Error>,
{
    match objectstore.get(path).await {
        Ok(get_result) => {
            let bytes = get_result.bytes().await?;
            info!(
                "fetch: {} -> received {} bytes ({})",
                path,
                bytes.len(),
                ByteSize(bytes.len() as u64)
            );
            block_in_place(move || f(bytes))
            //let data: T = block_in_place(move || deserialize_from_byte_slice(&bytes))?;
            //Ok(data)
        }
        Err(err) => match err {
            object_store::Error::NotFound { .. } => {
                info!("fetch: {} -> not found", path);
                Err(Error::from(err))
            }
            _ => {
                error!("fetch: {} -> {}", path, err.to_string());
                Err(Error::from(err))
            }
        },
    }
}

static RE_S3KEY_DATA_H3_RESOLUTION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\s*data_h3_resolution\s*\}").unwrap());
static RE_S3KEY_FILE_H3_RESOLUTION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\s*file_h3_resolution\s*\}").unwrap());
static RE_S3KEY_H3_CELL: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\s*h3cell\s*\}").unwrap());

fn build_dataset_path(
    dataset: &DataframeDataset,
    cell: &H3Cell,
    data_h3_resolution: u8,
) -> Result<Path, Error> {
    Ok(RE_S3KEY_H3_CELL
        .replace_all(
            &RE_S3KEY_FILE_H3_RESOLUTION.replace_all(
                &RE_S3KEY_DATA_H3_RESOLUTION
                    .replace_all(dataset.key_pattern.as_ref(), data_h3_resolution.to_string()),
                dataset.file_h3_resolution(data_h3_resolution)?.to_string(),
            ),
            cell.to_string(),
        )
        .to_string()
        .into())
}
