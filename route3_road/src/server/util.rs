//! utility functions to use within the server context, most of them
//! return a `tonic::Status` on error and a somewhat useful error message + logging.

use std::iter::FromIterator;

use arrow2::record_batch::RecordBatch;
use h3ron::Index;
use polars_core::frame::DataFrame;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use crate::io::dataframe::recordbatch_to_bytes;
use crate::server::api::generated::ArrowRecordBatch;

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

/// stream [`ArrowRecordBatch`] instances to a client
pub async fn respond_recordbatches_stream(
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
            tx.send(serialization_result).await.unwrap();
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}

pub fn extract_h3indexes<C, I>(dataframe: &DataFrame, column_name: &str) -> Result<C, Status>
where
    C: FromIterator<I>,
    I: Index,
{
    crate::io::dataframe::extract_h3indexes(dataframe, column_name).map_err(|e| {
        log::error!(
            "extracting indexes from column {} failed: {:?}",
            column_name,
            e
        );
        Status::invalid_argument(format!(
            "extracting indexes from column {} failed",
            column_name
        ))
    })
}
