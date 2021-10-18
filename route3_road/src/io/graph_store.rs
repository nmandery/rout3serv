use std::io::Cursor;
use std::sync::Arc;

use eyre::Result;
use h3ron::io::deserialize_from;
use h3ron_graph::graph::PreparedH3EdgeGraph;
use lru::LruCache;
use regex::Regex;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tokio::task::block_in_place;

use crate::config::GraphStoreConfig;
use crate::io::s3::{FoundOption, S3Client};

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

/// a graphcache
pub struct GraphStore<W: Send + Sync> {
    s3_client: Arc<S3Client>,
    graph_store_config: GraphStoreConfig,
    cache: Arc<Mutex<LruCache<GraphCacheKey, Arc<PreparedH3EdgeGraph<W>>>>>,
}

impl<W: Send + Sync> GraphStore<W>
where
    W: DeserializeOwned,
{
    pub fn new(s3_client: Arc<S3Client>, graph_store_config: GraphStoreConfig) -> Self {
        let cache = Arc::new(Mutex::new(LruCache::new(
            graph_store_config.cache_size.unwrap_or(4),
        )));
        Self {
            s3_client,
            graph_store_config,
            cache,
        }
    }

    pub async fn list(&self) -> Result<Vec<GraphCacheKey>> {
        let prefix_re = Regex::new(&format!(
            "^{}",
            regex::escape(self.graph_store_config.prefix.as_str())
        ))?;

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

    /// get a graph from the cache if it is available
    pub async fn load_cached(
        &self,
        graph_cache_key: &GraphCacheKey,
    ) -> Option<Arc<PreparedH3EdgeGraph<W>>> {
        // attempt to get the graph from the cache
        let mut guard = self.cache.lock().await;
        guard.get(graph_cache_key).cloned()
    }

    /// get a graph from the cache or from remote when its not loaded in the cache
    pub async fn load(
        &self,
        graph_cache_key: &GraphCacheKey,
    ) -> Result<Option<Arc<PreparedH3EdgeGraph<W>>>> {
        // attempt to get the graph from the cache
        if let Some(graph) = self.load_cached(graph_cache_key).await {
            return Ok(Some(graph));
        }

        let s3_key = format!(
            "{}{}",
            self.graph_store_config.prefix,
            gck_to_filename(graph_cache_key)
        );
        // could not get the graph from the cache, so try to fetch it
        match self
            .s3_client
            .get_object_bytes(self.graph_store_config.bucket.clone(), s3_key.clone())
            .await?
        {
            FoundOption::Found(graph_bytes) => {
                let graph: Arc<PreparedH3EdgeGraph<W>> = Arc::new(block_in_place(move || {
                    deserialize_from(Cursor::new(graph_bytes))
                })?);
                let mut guard = self.cache.lock().await;
                guard.put(graph_cache_key.clone(), graph.clone());
                Ok(Some(graph))
            }
            FoundOption::NotFound => {
                log::warn!("could not find graph {:?}", graph_cache_key);
                Ok(None)
            }
        }
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
