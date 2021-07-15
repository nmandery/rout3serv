use std::hash::BuildHasherDefault;

use ahash::AHasher;
pub use hashbrown::{HashMap, HashSet};
use indexmap::map::IndexMap;

use crate::h3ron::{H3Cell, H3Edge};

pub type H3EdgeMap<V> = HashMap<H3Edge, V>;
pub type H3CellMap<V> = HashMap<H3Cell, V>;
pub type H3CellSet = HashSet<H3Cell>;
pub type AIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<AHasher>>;
