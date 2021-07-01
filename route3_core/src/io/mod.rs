use std::fs::File;
use std::ops::Add;

use bytesize::ByteSize;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::graph::H3Graph;

#[cfg(feature = "with-gdal")]
pub mod gdal;

pub fn load_graph_from_byte_slice<'de, T>(slice: &'de [u8]) -> Result<H3Graph<T>, Error>
where
    T: PartialOrd + PartialEq + Add + Copy + Deserialize<'de>,
{
    log::debug!(
        "Deserializing graph. {} bytes ({})",
        slice.len(),
        ByteSize(slice.len() as u64)
    );
    let graph: H3Graph<T> = bincode::deserialize(slice)?;
    log::debug!(
        "Stats of the deserialized graph: {}",
        serde_json::to_string(&graph.stats()?)?
    );
    Ok(graph)
}

pub fn load_graph<R: std::io::Read, T>(mut reader: R) -> Result<H3Graph<T>, Error>
where
    T: PartialOrd + PartialEq + Add + Copy + DeserializeOwned,
{
    let mut raw_data: Vec<u8> = Default::default();
    reader.read_to_end(&mut raw_data)?;
    load_graph_from_byte_slice(raw_data.as_slice())

    /*
    let br = BufReader::new(reader);
    let graph: H3Graph<T> = bincode::deserialize_from(br)?;
    Ok(graph)
     */
}

pub fn save_graph_to_file<T>(graph: &H3Graph<T>, out_file: &mut File) -> Result<(), Error>
where
    T: PartialOrd + PartialEq + Add + Copy + Serialize,
{
    bincode::serialize_into(out_file, graph)?;
    Ok(())
}
