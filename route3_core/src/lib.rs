//!
//! # Hashing
//!
//! This crate uses `ahash` for its HashMap and HashSets. This hash hash shown in benchmarks to be
//! approx. 10% faster with H3 indexes than the standard SipHash used in rust. On the other hand it shows a higher
//! fluctuation in runtime during benchmarks. Interestingly the normally very fast
//! `rustc_hash` (uses `FxHash`) seems to be very slow with H3 cells and edges. Mostly noticed during
//! deserialization of graphs, but also during using the `pathfinding` crate which uses
//! `rustc_hash` internally. May be related to https://github.com/rust-lang/rustc-hash/issues/14
//!
// re-export core libraries for easier dependency management
#[cfg(feature = "with-gdal")]
pub use gdal;
pub use geo_types;
pub use h3ron;
use hashbrown::{HashMap, HashSet};

#[cfg(feature = "osm")]
pub use osmpbfreader;

use crate::h3ron::{H3Cell, H3Edge};
use ahash::AHasher;
use indexmap::map::IndexMap;
use std::hash::BuildHasherDefault;

mod algo;
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
type AIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<AHasher>>;

pub trait WithH3Resolution {
    fn h3_resolution(&self) -> u8;
}
