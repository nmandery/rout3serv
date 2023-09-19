use std::marker::PhantomData;

use roaring::RoaringTreemap;

#[cfg(feature = "serde")]
pub mod serde;

/// wrapper around [`roaring::RoaringTreemap`] to store h3 data.
///
/// The implementation of `roaring::RoaringTreemap` splits `u64` into two
/// `u32`. The first is used as the key for a `BTreeMap`, the second is used
/// in the bitmap value of that map. Due to the structure of h3 indexes, relevant parts
/// are only stored in the bitmap starting with approx h3 resolution 5. Below that it
/// makes little sense to use this `H3Treemap`.
#[derive(Clone)]
pub struct H3Treemap<T> {
    treemap: RoaringTreemap,
    phantom_data: PhantomData<T>,
}

impl<T> FromIterator<T> for H3Treemap<T>
where
    T: Into<u64> + Copy,
{
    /// Create from an iterator.
    ///
    /// create this struct from an iterator. The iterator is consumed and sorted in memory
    /// before creating the Treemap - this can greatly reduce the creation time.
    ///
    /// Requires accumulating the whole iterator in memory for a short while.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        // pre-sort for improved creation-speed of the RoaringTreemap
        let mut h3indexes: Vec<_> = iter
            .into_iter()
            .map(|c| {
                let v: u64 = c.into();
                v
            })
            .collect();
        h3indexes.sort_unstable();
        h3indexes.dedup();

        Self {
            treemap: RoaringTreemap::from_sorted_iter(h3indexes).unwrap(),
            phantom_data: Default::default(),
        }
    }
}

impl<T> Default for H3Treemap<T> {
    fn default() -> Self {
        Self {
            treemap: Default::default(),
            phantom_data: Default::default(),
        }
    }
}

impl<T> H3Treemap<T>
where
    T: Copy + Into<u64>,
{
    /// Pushes value in the treemap only if it is greater than the current maximum value.
    /// Returns whether the value was inserted.
    #[inline]
    pub fn push(&mut self, index: T) -> bool {
        let v: u64 = index.into();
        self.treemap.push(v)
    }

    /// Adds a value to the set. Returns true if the value was not already present in the set.
    #[inline]
    pub fn insert(&mut self, index: T) -> bool {
        let v: u64 = index.into();
        self.treemap.insert(v)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.treemap.len() as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.treemap.is_empty()
    }

    #[inline]
    pub fn contains(&self, index: &T) -> bool {
        let v: u64 = (*index).into();
        self.treemap.contains(v)
    }

    #[inline]
    pub fn is_disjoint(&self, rhs: &Self) -> bool {
        self.treemap.is_disjoint(&rhs.treemap)
    }

    #[inline]
    pub fn is_subset(&self, rhs: &Self) -> bool {
        self.treemap.is_subset(&rhs.treemap)
    }

    #[inline]
    pub fn is_superset(&self, rhs: &Self) -> bool {
        self.treemap.is_superset(&rhs.treemap)
    }
}

impl<T> H3Treemap<T>
where
    T: Copy + TryFrom<u64>,
{
    pub fn iter(&self) -> impl Iterator<Item = Result<T, T::Error>> + '_ {
        self.treemap.iter().map(|v| T::try_from(v))
    }
}

#[cfg(test)]
mod tests {
    use super::H3Treemap;
    use h3o::CellIndex;

    #[test]
    fn iter() {
        let idx = CellIndex::try_from(0x89283080ddbffff_u64).unwrap();
        let treemap: H3Treemap<_> = idx.grid_disk(1);
        assert_eq!(treemap.len(), 7);
    }
}
