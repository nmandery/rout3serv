use crate::h3ron::H3Edge;
use std::collections::HashMap;

// re-export core libraries for easier dependency management
pub use geo_types;
pub use h3ron;

pub mod graph;
pub mod io;
pub mod serde;

pub type H3EdgeMap<V> = HashMap<H3Edge, V>;

/*
Notes:

* rustc_hash (FxHashSet, FxHashMap) is really slow with H3Cell and H3Edge keys. std::collections::HashMap
  performs far better.

*/
