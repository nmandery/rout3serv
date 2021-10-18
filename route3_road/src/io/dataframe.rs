use std::iter::FromIterator;

use arrow2::io::ipc::write::FileWriter;
use arrow2::record_batch::RecordBatch;
use eyre::Result;
use h3ron::Index;
use polars_core::frame::DataFrame;

/// extract a column of a dataframe into a collection of ´Index´-implementing values.
pub fn extract_h3indexes<C, I>(dataframe: &DataFrame, column_name: &str) -> Result<C>
where
    C: FromIterator<I>,
    I: Index,
{
    let h3index_chunked = dataframe.column(column_name)?.u64()?;
    let collection = h3index_chunked
        .into_iter()
        .filter_map(|h3index_opt| {
            match h3index_opt {
                Some(h3index) => {
                    let index = I::from_h3index(h3index);
                    if index.is_valid() {
                        Some(index)
                    } else {
                        // simply ignore invalid h3indexes for now
                        None
                    }
                }
                None => None,
            }
        })
        .collect::<C>();
    Ok(collection)
}

/// serialize a [`RecordBatch`] into arrow IPC format
pub fn recordbatch_to_bytes(recordbatch: &RecordBatch) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = vec![];
    {
        let mut filewriter = FileWriter::try_new(&mut buf, &*recordbatch.schema())?;
        filewriter.write(recordbatch)?;
        filewriter.finish()?;
    }
    Ok(buf)
}
