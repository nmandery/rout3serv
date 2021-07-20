use std::any::type_name;
use std::any::Any;

use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;
use eyre::{Report, Result};

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
