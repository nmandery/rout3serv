//! utility functions to use within the server context, most of them
//! return a `tonic::Status` on error and a somewhat useful error message + logging.

use std::iter::FromIterator;

use h3ron::iter::change_resolution;
use h3ron::{H3Cell, Index};
use polars_core::frame::DataFrame;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Response, Status};

use s3io::dataframe::{dataframe_to_bytes, H3DataFrame};

use crate::server::api::generated::ArrowIpcChunk;
use crate::server::api::Route;
use crate::server::error::ToStatusResult;

/// wrapper around tokios `spawn_blocking` to directly
/// return the `JoinHandle` as a tonic `Status`.
pub async fn spawn_blocking_status<F, R>(f: F) -> Result<R, Status>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .to_status_message_result(Code::Internal, || {
            "joining blocking task failed".to_string()
        })
}

pub trait StrId {
    fn id(&self) -> &str;
}

/// type for a stream of ArrowRecordBatches to a GRPC client
pub type ArrowIpcChunkStream = ReceiverStream<Result<ArrowIpcChunk, Status>>;

/// stream `RouteWKB` instances
pub async fn stream_routes<R>(
    mut routewkbs: Vec<R>,
) -> Result<Response<ReceiverStream<Result<R, Status>>>, Status>
where
    R: Route + Send + 'static,
{
    let (tx, rx) = mpsc::channel(5);
    tokio::spawn(async move {
        for routewkb in routewkbs.drain(..) {
            if let Err(e) = tx.send(Ok(routewkb)).await {
                log::warn!("Streaming routes aborted. reason: {}", e);
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
    h3dataframe
        .index_collection()
        .to_status_message_result(Code::InvalidArgument, || {
            format!(
                "extracting {} from column {} failed",
                std::any::type_name::<I>(),
                h3dataframe.h3index_column_name,
            )
        })
}

pub fn change_cell_resolution_dedup(
    cells: &[H3Cell],
    h3_resolution: u8,
) -> Result<Vec<H3Cell>, Status> {
    let mut out_cells = change_resolution(cells, h3_resolution)
        .collect::<Result<Vec<_>, _>>()
        .to_status_result(Code::Internal)?;
    out_cells.sort_unstable();
    out_cells.dedup();
    Ok(out_cells)
}

#[inline]
pub async fn stream_dataframe(
    id: String,
    dataframe: DataFrame,
) -> Result<Response<ArrowIpcChunkStream>, Status> {
    stream_dataframe_with_max_rows(id, dataframe, 3000).await
}

/// respond with a dataframe as a stream of size limited Arrow IPC chunks.
///
/// slices dataframe into a fixed size of max `max_rows` rows
/// to stay within GRPCs message size limits.
pub async fn stream_dataframe_with_max_rows(
    id: String,
    dataframe: DataFrame,
    max_rows: usize,
) -> Result<Response<ArrowIpcChunkStream>, Status> {
    let df_shape = dataframe.shape();
    log::debug!(
        "responding with a dataframe {:?} as a stream of chunks (max rows = {})",
        df_shape,
        max_rows
    );

    let num_rows = df_shape.0;
    let mut dataframe_parts = Vec::with_capacity(num_rows / max_rows);
    let mut i: usize = 0;
    loop {
        let offset = i * max_rows;
        if offset >= num_rows {
            break;
        }
        dataframe_parts.push(dataframe.slice(offset as i64, max_rows));
        i += 1;
    }

    let (tx, rx) = mpsc::channel(5);
    tokio::spawn(async move {
        for df_part in dataframe_parts.drain(..) {
            let serialization_result = dataframe_to_bytes(&df_part)
                .to_status_message_result(Code::Internal, || {
                    "serializing dataframe failed".to_string()
                })
                .map(|ipc_bytes| ArrowIpcChunk {
                    object_id: id.clone(),
                    data: ipc_bytes,
                });
            if let Err(e) = tx.send(serialization_result).await {
                log::warn!("Streaming dataframe parts aborted. reason: {}", e);
                break;
            }
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}