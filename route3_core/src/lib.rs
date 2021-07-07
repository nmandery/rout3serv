// re-export core libraries for easier dependency management
#[cfg(feature = "with-gdal")]
pub use gdal;
pub use geo_types;
pub use h3ron;
use hashbrown::{HashMap, HashSet};

#[cfg(feature = "osm")]
pub use osmpbfreader;

use crate::h3ron::{H3Cell, H3Edge};

pub mod error;
#[cfg(feature = "with-gdal")]
pub mod gdal_util;
pub mod graph;
pub mod io;
pub mod iter;
#[cfg(feature = "osm")]
pub mod osm;
pub mod routing;

pub type H3EdgeMap<V> = HashMap<H3Edge, V>;
pub type H3CellMap<V> = HashMap<H3Cell, V>;
pub type H3CellSet = HashSet<H3Cell>;

/*
Notes:

* rustc_hash (FxHashSet, FxHashMap) is really slow with H3Cell and H3Edge keys. std::collections::HashMap
  performs far better. Mostly noticed during deserialization of Graphs. Related: https://github.com/rust-lang/rustc-hash/issues/14

*/

pub trait WithH3Resolution {
    fn h3_resolution(&self) -> u8;
}
