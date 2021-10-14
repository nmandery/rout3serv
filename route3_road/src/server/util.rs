//! utility functions to use within the server context, most of them
//! return a `tonic::Status` on error.

use arrow2::record_batch::RecordBatch;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

use crate::io::recordbatch_to_bytes;
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
/// TODO: should be in ::api
pub type ArrowRecordBatchStream = ReceiverStream<Result<ArrowRecordBatch, Status>>;

/// stream [`ArrowRecordBatch`] instances to a client
/// TODO: should be in ::api
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
