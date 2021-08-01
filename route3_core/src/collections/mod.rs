use hashbrown;
use indexmap;

pub use ahash::RandomState;
pub use partitioned::ThreadPartitionedMap;

use crate::h3ron::{H3Cell, H3Edge};

mod partitioned;

pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;
pub type HashSet<V> = hashbrown::HashSet<V, RandomState>;
pub type H3EdgeMap<V> = HashMap<H3Edge, V>;
pub type H3CellMap<V> = HashMap<H3Cell, V>;
pub type H3CellSet = HashSet<H3Cell>;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, RandomState>;
