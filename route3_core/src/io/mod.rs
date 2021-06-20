use std::fs::File;
use std::io::Write;
use std::ops::Add;

use bytesize::ByteSize;
use eyre::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::graph::H3Graph;

#[cfg(feature = "gdal")]
pub mod gdal;

pub fn load_graph_from_byte_slice<'de, T>(slice: &'de [u8]) -> Result<H3Graph<T>>
where
    T: Copy + Add + PartialOrd + PartialEq + Deserialize<'de>,
{
    log::debug!(
        "Deserializing graph. {} bytes ({})",
        slice.len(),
        ByteSize(slice.len() as u64)
    );
    let fx_reader = flexbuffers::Reader::get_root(slice)?;
    let graph = H3Graph::deserialize(fx_reader)?;
    log::debug!(
        "Stats of the deserialized graph: {}",
        serde_json::to_string(&graph.stats())?
    );
    Ok(graph)
}

pub fn load_graph<R: std::io::Read, T>(mut reader: R) -> Result<H3Graph<T>>
where
    T: Copy + Add + PartialOrd + PartialEq + DeserializeOwned,
{
    let mut raw_data: Vec<u8> = Default::default();
    reader.read_to_end(&mut raw_data)?;
    load_graph_from_byte_slice(raw_data.as_slice())
}

pub fn save_graph_to_file<T: Serialize>(graph: &H3Graph<T>, out_file: &mut File) -> Result<()>
where
    T: Copy + Add + PartialOrd + PartialEq,
{
    let mut serializer = flexbuffers::FlexbufferSerializer::new();
    graph.serialize(&mut serializer)?;
    out_file.write_all(serializer.view())?;
    out_file.flush()?;
    Ok(())
}
