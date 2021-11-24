//! utility functions to use within the server context, most of them
//! return a `tonic::Status` on error and a somewhat useful error message + logging.

use std::iter::FromIterator;

use arrow::record_batch::RecordBatch;
use h3ron::iter::change_cell_resolution;
use h3ron::{H3Cell, Index};
use polars_core::frame::DataFrame;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use crate::server::api::generated::ArrowRecordBatch;
use s3io::dataframe::{recordbatch_to_bytes, H3DataFrame};

/// wrapper around tokios `spawn_blocking` to directly
/// return the `JoinHandle` as a tonic `Status`.
pub async fn spawn_blocking_status<F, R>(f: F) -> Result<R, Status>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| {
        log::error!("joining blocking task failed: {:?}", e);
        Status::internal("join error")
    })
}

#[inline]
fn recordbatch_to_bytes_status(recordbatch: &RecordBatch) -> Result<Vec<u8>, Status> {
    let recordbatch_bytes = recordbatch_to_bytes(recordbatch).map_err(|e| {
        log::error!("serializing recordbatch failed: {:?}", e);
        Status::internal("serializing recordbatch failed")
    })?;
    Ok(recordbatch_bytes)
}

pub trait StrId {
    fn id(&self) -> &str;
}

/// type for a stream of ArrowRecordBatches to a GRPC client
pub type ArrowRecordBatchStream = ReceiverStream<Result<ArrowRecordBatch, Status>>;

/// respond with a dataframe as a stream of `RecordBatch` instances.
pub async fn stream_dataframe(
    id: String,
    dataframe: DataFrame,
) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
    let df_shape = dataframe.shape();
    let recordbatches = to_streamable_recordbatches(
        dataframe.as_record_batches().map_err(|e| {
            log::error!("could not convert dataframe to recordbatches: {:?}", e);
            Status::internal("could not convert dataframe to recordbatches")
        })?,
        3000,
    )?;
    log::debug!(
        "responding with a dataframe {:?} as a stream of {} recordbatches",
        df_shape,
        recordbatches.len()
    );
    stream_recordbatches(id, recordbatches).await
}

/// stream [`ArrowRecordBatch`] instances to a client.
///
/// Depending on the size of the batches, passing them through `to_streamable_recordbatches`
/// may be recommended.
async fn stream_recordbatches(
    id: String,
    mut recordbatches: Vec<RecordBatch>,
) -> std::result::Result<Response<ArrowRecordBatchStream>, Status> {
    let (tx, rx) = mpsc::channel(5);
    tokio::spawn(async move {
        for recordbatch in recordbatches.drain(..) {
            let serialization_result =
                recordbatch_to_bytes_status(&recordbatch).map(|rb_bytes| ArrowRecordBatch {
                    object_id: id.clone(),
                    data: rb_bytes,
                });
            if let Err(e) = tx.send(serialization_result).await {
                log::warn!("Streaming recordbatches aborted. reason: {}", e);
                break;
            }
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}

pub fn index_collection_from_h3dataframe<C, I>(h3dataframe: &H3DataFrame) -> Result<C, Status>
where
    C: FromIterator<I>,
    I: Index,
{
    h3dataframe.index_collection().map_err(|e| {
        log::error!(
            "extracting {} from column {} failed: {:?}",
            std::any::type_name::<I>(),
            h3dataframe.h3index_column_name,
            e
        );
        Status::invalid_argument(format!(
            "extracting indexes from column {} failed",
            h3dataframe.h3index_column_name
        ))
    })
}

pub fn change_cell_resolution_dedup(cells: &[H3Cell], h3_resolution: u8) -> Vec<H3Cell> {
    let mut out_cells: Vec<_> = change_cell_resolution(cells, h3_resolution).collect();
    out_cells.sort_unstable();
    out_cells.dedup();
    out_cells
}

/// slice recordbatches into a fixed size of max `max_rows` rows
/// to stay within GRPCs message size limits.
fn to_streamable_recordbatches(
    recordbatches: Vec<RecordBatch>,
    max_rows: usize,
) -> Result<Vec<RecordBatch>, Status> {
    let mut new_recordbatches = Vec::new();
    for rb in recordbatches {
        if rb.num_rows() <= max_rows {
            new_recordbatches.push(rb);
        } else {
            let mut i = 0_usize;
            while i * max_rows < rb.num_rows() {
                let offset = i * max_rows;
                let length = (rb.num_rows() - offset).min(max_rows);

                let columns = rb
                    .columns()
                    .iter()
                    .map(|column| column.slice(offset, length).into())
                    .collect();

                let new_rb = RecordBatch::try_new(rb.schema().clone(), columns).map_err(|e| {
                    log::error!("slicing up recordbatches failed: {:?}", e);
                    Status::internal("slicing recordbatches failed")
                })?;
                new_recordbatches.push(new_rb);
                i += 1;
            }
        }
    }
    Ok(new_recordbatches)
}
