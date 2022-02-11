use std::io::Cursor;
use std::marker::PhantomData;
use std::sync::Arc;

use h3ron_graph::graph::PreparedH3EdgeGraph;
use regex::Regex;
use s3io::ser_and_de::deserialize_from;
use serde::de::DeserializeOwned;
use tokio::task::block_in_place;

use s3io::fetch::{AsyncFetcher, FetchCache, FetchError};
use s3io::s3::{ObjectRef, S3Client};
use s3io::Error;

use crate::config::GraphStoreConfig;

const GRAPH_SUFFIX: &str = ".bincode.lz";

lazy_static! {
    static ref RE_GRAPH_FILE: Regex = {
        let graph_re_string: String = format!(
            "(?P<name>[a-zA-Z0-9\\-_]+)_(?P<h3_res>[0-9]?[0-9]){}$",
            regex::escape(GRAPH_SUFFIX)
        );
        Regex::new(&graph_re_string).unwrap()
    };
}
#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct GraphCacheKey {
    pub name: String,
    pub h3_resolution: u8,
}

fn gck_to_filename(gck: &GraphCacheKey) -> String {
    format!("{}_{}{}", gck.name, gck.h3_resolution, GRAPH_SUFFIX)
}

fn filename_to_gck(filename: &str) -> Option<GraphCacheKey> {
    RE_GRAPH_FILE.captures(filename).map(|cap| GraphCacheKey {
        name: cap.name("name").unwrap().as_str().to_string(),
        h3_resolution: cap.name("h3_res").unwrap().as_str().parse().unwrap(),
    })
}

struct GraphFetcher<W: Send + Sync>
where
    W: DeserializeOwned,
{
    phantom_weight: PhantomData<W>,
    s3_client: Arc<S3Client>,
}

#[async_trait::async_trait]
impl<W: Send + Sync> AsyncFetcher for GraphFetcher<W>
where
    W: DeserializeOwned,
{
    type Key = ObjectRef;
    type Value = PreparedH3EdgeGraph<W>;
    type Error = s3io::Error;

    async fn fetch(&self, key: Self::Key) -> Result<Self::Value, Self::Error> {
        let graph_bytes = self.s3_client.get_object_bytes(key).await?;
        let graph: PreparedH3EdgeGraph<W> =
            block_in_place(move || deserialize_from(Cursor::new(graph_bytes)))?;
        Ok(graph)
    }
}

/// a graphcache
pub struct GraphStore<W: Send + Sync>
where
    W: DeserializeOwned,
{
    s3_client: Arc<S3Client>,
    graph_store_config: GraphStoreConfig,
    fetch_cache: FetchCache<GraphFetcher<W>>,
}

impl<W: Send + Sync> GraphStore<W>
where
    W: DeserializeOwned,
{
    pub fn new(s3_client: Arc<S3Client>, graph_store_config: GraphStoreConfig) -> Self {
        let fetch_cache = FetchCache::new(
            graph_store_config.cache_size.unwrap_or(4),
            GraphFetcher {
                phantom_weight: Default::default(),
                s3_client: s3_client.clone(),
            },
        );
        Self {
            s3_client,
            graph_store_config,
            fetch_cache,
        }
    }

    pub async fn list(&self) -> Result<Vec<GraphCacheKey>, Error> {
        let prefix_re = Regex::new(&format!(
            "^{}",
            regex::escape(self.graph_store_config.prefix.as_str())
        ))
        .map_err(|e| Error::Generic(format!("escaping regex failed: {:?}", e)))?;

        let keys = self
            .s3_client
            .list_object_keys(
                self.graph_store_config.bucket.clone(),
                Some(self.graph_store_config.prefix.clone()),
            )
            .await?
            .drain(..)
            .filter_map(|key| filename_to_gck(&*prefix_re.replace_all(&key, "")))
            .collect();
        Ok(keys)
    }

    /// get a graph from the cache or from remote when its not loaded in the cache
    pub async fn load(
        &self,
        graph_cache_key: &GraphCacheKey,
    ) -> Result<Arc<PreparedH3EdgeGraph<W>>, FetchError<s3io::Error>> {
        let s3_key = format!(
            "{}{}",
            self.graph_store_config.prefix,
            gck_to_filename(graph_cache_key)
        );
        self.fetch_cache
            .get(ObjectRef::new(
                self.graph_store_config.bucket.clone(),
                s3_key,
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{filename_to_gck, GraphCacheKey};

    #[test]
    fn graph_regex() {
        assert_eq!(
            filename_to_gck("somegraph_7.bincode.lz"),
            Some(GraphCacheKey {
                name: "somegraph".to_string(),
                h3_resolution: 7,
            })
        );
    }
}
