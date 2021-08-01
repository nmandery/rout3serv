use std::borrow::BorrowMut;
use std::cmp::max;
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter::FromIterator;

use rayon::prelude::*;

use super::RandomState;

/// goal: populating the map faster from serialized data
pub struct ThreadPartitionedMap<K, V, S = RandomState> {
    build_hasher: S,
    partitions: Vec<hashbrown::HashMap<K, V, S>>,
}

impl<K, V, S> ThreadPartitionedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
    S: BuildHasher + Default + Send + Clone,
{
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let num_partitions = max(num_cpus::get().saturating_sub(1), 4);
        let partition_capacity = if capacity > 0 {
            // expecting an equal distribution of keys over all partitions
            1 + capacity / num_partitions
        } else {
            0
        };
        let build_hasher = S::default();
        let partitions = (0..num_partitions)
            .map(|_| {
                hashbrown::HashMap::with_capacity_and_hasher(
                    partition_capacity,
                    // all partitions must use the hasher with the same seed to generate the same
                    // hashes
                    build_hasher.clone(),
                )
            })
            .collect();

        Self {
            build_hasher,
            partitions,
        }
    }

    pub fn num_partitions(&self) -> usize {
        self.partitions.len()
    }

    pub fn len(&self) -> usize {
        self.partitions.iter().map(|p| p.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.partitions.iter().all(|p| p.is_empty())
    }

    pub fn insert_many<I>(&mut self, iter: I)
    where
        I: Iterator<Item = (K, V)>,
    {
        self.insert_or_modify_many(iter, |_old, new| new);
    }

    ///
    /// `modify_fn` takes two values and creates the value to be stored. The first value
    /// is the one which was previously in the map, the second one is the new one.
    pub fn insert_or_modify_many<I, F>(&mut self, iter: I, modify_fn: F)
    where
        I: Iterator<Item = (K, V)>,
        F: Fn(V, V) -> V + Sync,
    {
        let num_partitions = self.num_partitions() as u64;
        let hashed_kv = hash_vectorized(iter, &self.build_hasher, num_partitions as usize);
        let new_partitions = std::mem::take(&mut self.partitions)
            .par_drain(..)
            .zip(hashed_kv)
            .map(|(mut partition, partition_hashed_kv)| {
                for (h, (k, v)) in partition_hashed_kv.iter() {
                    // raw_entry_mut in `std` requires nightly. in hashbrown it is already stable
                    // https://github.com/rust-lang/rust/issues/56167
                    let raw_entry = partition.raw_entry_mut().from_key_hashed_nocheck(*h, k);

                    match raw_entry {
                        hashbrown::hash_map::RawEntryMut::Occupied(mut entry) => {
                            let (_occupied_key, occupied_value) = entry.get_key_value_mut();
                            *occupied_value = modify_fn(*occupied_value, *v)
                        }
                        hashbrown::hash_map::RawEntryMut::Vacant(entry) => {
                            entry.insert_hashed_nocheck(*h, *k, *v);
                        }
                    }
                }
                partition
            })
            .collect();
        self.partitions = new_partitions;
    }

    fn key_partition(&self, key: &K) -> (u64, usize) {
        let mut hasher = self.build_hasher.build_hasher();
        key.hash(&mut hasher);
        let h = hasher.finish();
        (h, h_partition(h, self.num_partitions() as u64) as usize)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let (h, partition) = self.key_partition(key);
        self.partitions[partition]
            .raw_entry()
            .from_key_hashed_nocheck(h, key)
            .map(|(_, v)| v)
    }

    /*
    pub fn entry(&mut self, key: K) -> hashbrown::hash_map::Entry<K, V, S> {
        let partition = self.key_partition(&key);
        self.partitions[partition].entry(key)
    }

     */

    pub fn keys(&self) -> TPMKeys<K, V, S> {
        TPMKeys {
            tpm: self,
            current_partition: 0,
            current_keys_iter: None,
        }
    }

    pub fn iter(&self) -> TPMIter<K, V, S> {
        TPMIter {
            tpm: self,
            current_partition: 0,
            current_iter: None,
        }
    }

    pub fn drain(&mut self) -> TPMDrain<'_, K, V> {
        let num_elements = self.len();
        let inner = self.partitions.iter_mut().map(|p| p.drain()).collect();
        TPMDrain {
            current: 0,
            inner,
            num_elements,
        }
    }
}

impl<K, V, S> Default for ThreadPartitionedMap<K, V, S>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
    S: BuildHasher + Default + Send + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> FromIterator<(K, V)> for ThreadPartitionedMap<K, V, RandomState>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let it = iter.into_iter();
        let mut tpm = Self::with_capacity(it.size_hint().0); // TODO: use upper or lower bound?
        tpm.insert_many(it);
        tpm
    }
}

fn hash_vectorized<K, V, S, I>(
    iter: I,
    build_hasher: &S,
    num_partitions: usize,
) -> Vec<Vec<(u64, (K, V))>>
where
    K: Hash,
    S: BuildHasher,
    I: Iterator<Item = (K, V)>,
{
    let iter_ = iter.into_iter();
    let mut out_vecs: Vec<_> = (0..num_partitions)
        .map(|_| Vec::with_capacity(iter_.size_hint().0 / num_partitions))
        .collect();

    iter_.for_each(|(k, v)| {
        let mut hasher = build_hasher.build_hasher();
        k.hash(&mut hasher);
        let h = hasher.finish();
        out_vecs[h_partition(h, num_partitions as u64) as usize].push((h, (k, v)));
    });
    out_vecs
}

/*
#[inline]
fn this_partition(h: u64, partition_number: u64, num_partitions: u64) -> bool {
    h_partition(h, num_partitions) == partition_number
}

 */

#[inline]
fn h_partition(h: u64, num_partitions: u64) -> u64 {
    // based on https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
    // and used instead of modulo (`h % num_partitions`)
    ((h as u128 * num_partitions as u128) >> 64) as u64
}

pub struct TPMKeys<'a, K, V, S> {
    tpm: &'a ThreadPartitionedMap<K, V, S>,
    current_partition: usize,
    current_keys_iter: Option<hashbrown::hash_map::Keys<'a, K, V>>,
}

impl<'a, K, V, S> Iterator for TPMKeys<'a, K, V, S>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
    S: BuildHasher + Default + Send + Clone,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current_keys_iter) = self.current_keys_iter.borrow_mut() {
                if let Some(next_key) = current_keys_iter.next() {
                    return Some(next_key);
                } else {
                    self.current_partition += 1;
                }
            }
            if let Some(partition) = self.tpm.partitions.get(self.current_partition) {
                self.current_keys_iter = Some(partition.keys())
            } else {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.tpm.len(), None)
    }
}

pub struct TPMIter<'a, K, V, S> {
    tpm: &'a ThreadPartitionedMap<K, V, S>,
    current_partition: usize,
    current_iter: Option<hashbrown::hash_map::Iter<'a, K, V>>,
}

impl<'a, K, V, S> Iterator for TPMIter<'a, K, V, S>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
    S: BuildHasher + Default + Send + Clone,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current_iter) = self.current_iter.borrow_mut() {
                if let Some(next_kv) = current_iter.next() {
                    return Some(next_kv);
                } else {
                    self.current_partition += 1;
                }
            }
            if let Some(partition) = self.tpm.partitions.get(self.current_partition) {
                self.current_iter = Some(partition.iter())
            } else {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.tpm.len(), None)
    }
}

pub struct TPMDrain<'a, K, V> {
    current: usize,
    inner: Vec<hashbrown::hash_map::Drain<'a, K, V>>,
    num_elements: usize,
}

impl<'a, K, V> Iterator for TPMDrain<'a, K, V>
where
    K: Hash + Eq + Send + Sync + Copy,
    V: Send + Sync + Copy,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(kv) = self.inner[self.current].next() {
                return Some(kv);
            }
            self.current += 1;
            if self.current >= self.inner.len() {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.num_elements, None)
    }
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use h3ron::H3Edge;

    use crate::collections::ThreadPartitionedMap;
    use crate::h3ron::Index;

    #[test]
    fn from_and_to_vec() {
        let in_vec: Vec<_> = (0_u64..1_000_000).map(|i| (H3Edge::new(i), i)).collect();
        let mut tpm = ThreadPartitionedMap::from_iter(in_vec.clone());
        assert_eq!(tpm.len(), 1_000_000);
        assert_eq!(tpm.get(&H3Edge::new(613777)), Some(&613777));
        let mut out_vec: Vec<_> = tpm.drain().collect();
        out_vec.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(in_vec, out_vec);
    }
}
