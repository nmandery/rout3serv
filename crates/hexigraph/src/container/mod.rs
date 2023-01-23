use ahash::RandomState;
use h3o::{CellIndex, DirectedEdgeIndex};

pub mod block;
pub mod treemap;

pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;
pub type HashSet<V> = hashbrown::HashSet<V, RandomState>;
pub type DirectedEdgeMap<V> = HashMap<DirectedEdgeIndex, V>;
pub type CellMap<V> = HashMap<CellIndex, V>;
pub type CellSet = HashSet<CellIndex>;
