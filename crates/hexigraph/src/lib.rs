use h3o::Resolution;

pub mod algorithm;
pub mod container;
pub mod error;
pub mod graph;
pub mod io;

/// trait to be implemented by all structs being based
/// on H3 data with a given resolution
pub trait HasH3Resolution {
    /// Gets the index resolution
    fn h3_resolution(&self) -> Resolution;
}
