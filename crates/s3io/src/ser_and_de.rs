//! deserialization utilities

use crate::Error;
use std::io;
use std::panic::{catch_unwind, UnwindSafe};

pub use h3ron_graph::io::serde_util::serialize_into;

pub fn deserialize_from<R, T>(reader: R) -> Result<T, Error>
where
    R: io::Read + io::Seek + UnwindSafe,
    T: serde::de::DeserializeOwned,
{
    // bincode may panic when encountering corrupt data
    let deserialized = catch_unwind(|| h3ron_graph::io::serde_util::deserialize_from(reader))
        .map_err(|_| Error::DeserializePanic)??;
    Ok(deserialized)
}
