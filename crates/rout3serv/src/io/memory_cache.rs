use std::error::Error;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

use async_trait::async_trait;
use indexmap::map::IndexMap;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tracing::debug;

use crate::io::objectstore::ObjectStore;

#[async_trait]
pub trait CacheFetcher {
    /// the key used to fetch the entry and to identify it.
    type Key;

    /// the type of the values to be cached.
    type Value;

    /// the error type returned by the fetcher.
    type Error;

    async fn fetch_from(
        &self,
        objectstore: Arc<ObjectStore>,
        key: Self::Key,
    ) -> Result<Self::Value, Self::Error>;

    /// checks an error if it is supposed to be cached as well.
    ///
    /// Default is no caching for errors
    fn is_cacheable_error(&self, _key: &Self::Key, _err: &Self::Error) -> bool {
        false
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError<E> {
    Fetch(Arc<E>),
    Recv(broadcast::error::RecvError),
}

#[derive(Clone)]
enum CacheEntry<V, E> {
    Available(Arc<V>),
    Error(Arc<E>),
    Fetching(broadcast::Sender<Result<Arc<V>, Arc<E>>>),
}

/// a cache wrapping a `CacheFetcher` to keep fetched values in memory.
///
/// Synchronises between multiple tasks to perform a fetch only once
/// even when teh value is requested from multiple tasks.
pub struct MemoryCache<F>
where
    F::Key: Eq + Hash + Clone + ToString,
    F: CacheFetcher,
    F::Error: Error,
{
    capacity: usize,
    fetcher: F,

    #[allow(clippy::type_complexity)]
    cache_map: Mutex<IndexMap<F::Key, CacheEntry<F::Value, F::Error>>>,
}

impl<F> MemoryCache<F>
where
    F::Key: Eq + Hash + Clone + ToString,
    F: CacheFetcher,
    F::Error: Error,
{
    pub fn new(capacity: usize, fetcher: F) -> Self {
        Self {
            capacity,
            fetcher,
            cache_map: Mutex::new(IndexMap::with_capacity(capacity + 1)),
        }
    }

    /// clear all cache contents
    #[allow(dead_code)]
    pub async fn cache_clear(&self) {
        let mut guard = self.cache_map.lock().await;
        guard.clear();
    }

    /// number of items in the cache
    #[allow(dead_code)]
    pub async fn cache_len(&self) -> usize {
        let guard = self.cache_map.lock().await;
        guard.len()
    }

    async fn insert_cache_entry(&self, key: F::Key, entry: CacheEntry<F::Value, F::Error>) {
        let mut guard = self.cache_map.lock().await;
        guard.insert(key, entry);

        // remove a few entries to stay within the capacity
        let mut i = 0;
        loop {
            if guard.len() < self.capacity || i >= guard.len() {
                break;
            }
            match guard.get_index(i) {
                Some((_, CacheEntry::Available(_))) | Some((_, CacheEntry::Error(_))) => {
                    guard.shift_remove_index(i); // remove this entry
                }
                _ => i += 1, // skip this entry as it still fetching
            }
        }
    }

    /// get a value from the cache or fetch it when it is not cached
    pub async fn get_from(
        &self,
        objectstore: Arc<ObjectStore>,
        key: F::Key,
    ) -> Result<Arc<F::Value>, FetchError<F::Error>> {
        let (tx, rx) = {
            let mut guard = self.cache_map.lock().await;

            // check if the value is already cached or the fetch is in progress
            if let Some(entry) = guard.get(&key) {
                match entry {
                    CacheEntry::Available(v) => {
                        debug!("cache hit (available) for {}", key.to_string());
                        return Ok(v.clone());
                    }
                    CacheEntry::Error(e) => {
                        debug!("cache hit (error) for {}", key.to_string());
                        return Err(FetchError::Fetch(e.clone()));
                    }
                    CacheEntry::Fetching(tx) => {
                        debug!("cache hit (fetching) for {}", key.to_string());
                        (None, Some(tx.subscribe()))
                    }
                }
            } else {
                // no fetch is in progress
                //
                // create a cache key containing the allow future `get` calls to obtain
                // a receiver for this fetch
                debug!("cache miss for {}", key.to_string());
                let (tx, _) = broadcast::channel(1);
                guard.insert(key.clone(), CacheEntry::Fetching(tx.clone()));
                (Some(tx), None)
            }
            // ... end of scope releases guard
        };

        match (tx, rx) {
            (Some(tx), None) => {
                let fetch_result = match self.fetcher.fetch_from(objectstore, key.clone()).await {
                    Ok(value) => {
                        let arc_value = Arc::new(value);
                        self.insert_cache_entry(key, CacheEntry::Available(arc_value.clone()))
                            .await;
                        Ok(arc_value)
                    }
                    Err(e) => {
                        let arc_e = Arc::new(e);
                        if self.fetcher.is_cacheable_error(&key, arc_e.as_ref()) {
                            self.insert_cache_entry(key, CacheEntry::Error(arc_e.clone()))
                                .await;
                        }
                        Err(arc_e)
                    }
                };
                // attempt to send, in case this errors, there are no receivers, so that
                // error can be ignored
                let _ = tx.send(fetch_result.clone());

                fetch_result.map_err(FetchError::Fetch)
            }

            (None, Some(mut rx)) => rx
                .recv()
                .await
                .map_err(FetchError::Recv)?
                .map_err(FetchError::Fetch),

            _ => unreachable!(),
        }
    }

    pub fn inner(&self) -> &F {
        &self.fetcher
    }
}

impl<F> Deref for MemoryCache<F>
where
    F::Key: Eq + Hash + Clone + ToString,
    F: CacheFetcher,
    F::Error: Error,
{
    type Target = F;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use tokio::time::Duration;

    use crate::io::objectstore::{ObjectStore, ObjectStoreConfig};

    use super::{CacheFetcher, MemoryCache};

    struct MyFetcher {
        pub call_counter: Mutex<usize>,
    }

    impl MyFetcher {
        pub fn new() -> Self {
            Self {
                call_counter: Mutex::new(0),
            }
        }

        pub async fn call_count(&self) -> usize {
            let guard = self.call_counter.lock().await;
            *guard
        }
    }

    #[async_trait]
    impl CacheFetcher for MyFetcher {
        type Key = u8;
        type Value = u32;
        type Error = Error;

        async fn fetch_from(
            &self,
            _objectstore: Arc<ObjectStore>,
            key: Self::Key,
        ) -> Result<Self::Value, Self::Error> {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut guard = self.call_counter.lock().await;
            *guard += 1;
            Ok(key as u32)
        }
    }

    fn get_objectstore() -> Arc<ObjectStore> {
        Arc::new(
            ObjectStore::try_from(ObjectStoreConfig::Filesystem {
                root: ".".to_string(),
            })
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn two_requests_one_cached() {
        let os = get_objectstore();
        let cache = MemoryCache::new(10, MyFetcher::new());
        assert_eq!(cache.get_from(os.clone(), 5).await.unwrap(), Arc::new(5));
        assert_eq!(cache.get_from(os.clone(), 5).await.unwrap(), Arc::new(5)); // should be cached
        assert_eq!(cache.inner().call_count().await, 1);
    }

    #[tokio::test]
    async fn two_requests_none_cached() {
        let os = get_objectstore();
        let cache = MemoryCache::new(10, MyFetcher::new());
        assert_eq!(cache.get_from(os.clone(), 5).await.unwrap(), Arc::new(5));
        assert_eq!(cache.get_from(os.clone(), 6).await.unwrap(), Arc::new(6));
        assert_eq!(cache.inner().call_count().await, 2);
    }

    #[tokio::test]
    async fn concurrent_cached_requests() {
        let os = get_objectstore();
        let cache = Arc::new(MemoryCache::new(10, MyFetcher::new()));
        let mut handles = vec![];
        for _ in 0..20 {
            let os = os.clone();
            let c = cache.clone();
            handles.push(tokio::spawn(async move { c.get_from(os, 11).await }));
        }
        for handle in futures::future::join_all(handles).await {
            handle.unwrap().unwrap();
        }
        assert_eq!(cache.inner().call_count().await, 1);
    }
}
