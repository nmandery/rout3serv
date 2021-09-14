use std::any::type_name;
use std::any::Any;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::{Read, Seek, Write};
use std::sync::Arc;

use arrow2::array::{Array, Float64Array, Float64Vec, UInt64Array, UInt64Vec};
use arrow2::datatypes::{DataType, Field, Schema};
use arrow2::io::ipc::read::{read_file_metadata, FileReader};
use arrow2::io::ipc::write::FileWriter;
use arrow2::record_batch::RecordBatch;
use eyre::{Report, Result};

use h3ron::{H3Edge, Index, H3_MAX_RESOLUTION};
use h3ron_graph::graph::H3EdgeGraph;

use crate::types::Weight;

pub mod s3;

/// get and downcast an array of a arrow recordbatch
pub fn recordbatch_array<'a, A: Any>(rb: &'a RecordBatch, column_name: &'a str) -> Result<&'a A> {
    let schema = rb.schema();
    let (idx, field) = schema
        .column_with_name(column_name)
        .ok_or_else(|| Report::msg(format!("recordbatch is missing the {} column", column_name)))?;

    let arr = rb.column(idx).as_any().downcast_ref::<A>().ok_or_else(|| {
        Report::msg(format!(
            "accessing column {} (type={}) as {} failed. wrong type",
            column_name,
            field.data_type().to_string(),
            type_name::<A>()
        ))
    })?;
    Ok(arr)
}

pub fn recordbatch_to_bytes(recordbatch: &RecordBatch) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = vec![];
    {
        let mut filewriter = FileWriter::try_new(&mut buf, &*recordbatch.schema())?;
        filewriter.write(recordbatch)?;
        filewriter.finish()?;
    }
    Ok(buf)
}

static ARROW_GRAPH_FIELD_EDGE: &str = "h3edge";
static ARROW_GRAPH_FIELD_WEIGHT: &str = "weight";
static ARROW_GRAPH_MD_RESOLUTION: &str = "h3_resolution";

pub fn arrow_save_graph<W>(graph: &H3EdgeGraph<Weight>, mut writer: W) -> Result<()>
where
    W: Write,
{
    let mut metadata = HashMap::new();
    metadata.insert(
        ARROW_GRAPH_MD_RESOLUTION.to_string(),
        graph.h3_resolution.to_string(),
    );

    let schema = Arc::new(Schema::new_from(
        vec![
            Field::new(ARROW_GRAPH_FIELD_EDGE, DataType::UInt64, false),
            Field::new(ARROW_GRAPH_FIELD_WEIGHT, DataType::Float64, false),
        ],
        metadata,
    ));
    let mut h3edges = UInt64Vec::with_capacity(graph.edges.len());
    let mut weights = Float64Vec::with_capacity(graph.edges.len());
    for (h3edge, weight) in graph.edges.iter() {
        h3edges.push(Some(h3edge.h3index() as u64));
        weights.push(Some(**weight));
    }

    let recordbatch = RecordBatch::try_new(schema, vec![h3edges.into_arc(), weights.into_arc()])?;

    let mut filewriter = FileWriter::try_new(&mut writer, recordbatch.schema())?;
    filewriter.write(&recordbatch)?;
    filewriter.finish()?;
    Ok(())
}

#[allow(dead_code)]
pub fn arrow_save_graph_bytes(graph: &H3EdgeGraph<Weight>) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = vec![];
    arrow_save_graph(graph, &mut buf)?;
    Ok(buf)
}

pub fn arrow_load_graph<R>(mut reader: R) -> Result<H3EdgeGraph<Weight>>
where
    R: Read + Seek,
{
    let metadata = read_file_metadata(&mut reader)?;
    let filereader = FileReader::new(&mut reader, metadata, None);
    let schema = filereader.schema();
    let h3_resolution = if let Some(h3res_string) = schema.metadata().get(ARROW_GRAPH_MD_RESOLUTION)
    {
        let h3_resolution = h3res_string.parse::<u8>().map_err(|_e| {
            Report::msg(format!(
                "Arrow file has invalid value for {}: '{}'",
                ARROW_GRAPH_MD_RESOLUTION, h3res_string
            ))
        })?;
        if h3_resolution > H3_MAX_RESOLUTION {
            return Err(Report::msg(format!(
                "Arrow file has an invalid h3 resolution ({})",
                h3res_string
            )));
        } else {
            h3_resolution
        }
    } else {
        return Err(Report::msg(format!(
            "Arrow file is missing the {} metadata field",
            ARROW_GRAPH_MD_RESOLUTION
        )));
    };

    let mut graph = H3EdgeGraph::new(h3_resolution);
    for recordbatch_result in filereader {
        let recordbatch = recordbatch_result?;
        let edges = recordbatch_array::<UInt64Array>(&recordbatch, ARROW_GRAPH_FIELD_EDGE)?;
        let weights = recordbatch_array::<Float64Array>(&recordbatch, ARROW_GRAPH_FIELD_WEIGHT)?;
        let mut validated_edges = Vec::with_capacity(edges.len());
        for option_tuple in edges.iter().zip(weights.iter()) {
            if let (Some(edge), Some(weight)) = option_tuple {
                let h3edge = H3Edge::try_from(*edge)?;
                if h3edge.resolution() != h3_resolution {
                    return Err(Report::msg(format!(
                        "Encountered h3edge with unexpected resolution {}. Expected was {}",
                        h3edge.resolution(),
                        h3_resolution
                    )));
                }
                validated_edges.push((h3edge, Weight::from(*weight)));
            }
        }
        graph.edges.insert_many(validated_edges.drain(..))
    }
    Ok(graph)
}
