//! utility functions to use within the grpc context, most of them
//! return a `tonic::Status` on error and a somewhat useful error message + logging.

use h3o::{CellIndex, Resolution};
use hexigraph::algorithm::resolution::transform_resolution;
use itertools::Itertools;
use polars::prelude::{DataFrame, DataFrameJoinOps, IpcWriter, JoinType, SerWriter};
use polars_core::prelude::JoinArgs;
use tokio::sync::mpsc;
use tokio::task::block_in_place;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Code, Response, Status};
use tracing::{debug, warn};

use crate::grpc::api::generated::ArrowIpcChunk;
use crate::grpc::api::Route;
use crate::grpc::error::ToStatusResult;
use crate::io::dataframe::CellDataFrame;

/// wrapper around tokios `spawn_blocking` to directly
/// return the `JoinHandle` as a tonic `Status`.
pub async fn spawn_blocking_status<F, R>(f: F) -> Result<R, Status>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.to_status_result()
}

pub trait StrId {
    fn id(&self) -> &str;
}

/// type for a stream of ArrowRecordBatches to a GRPC client
pub type ArrowIpcChunkStream = ReceiverStream<Result<ArrowIpcChunk, Status>>;

/// stream `RouteWKB` instances
pub async fn stream_routes<R>(
    routewkbs: Vec<R>,
) -> Result<Response<ReceiverStream<Result<R, Status>>>, Status>
where
    R: Route + Send + 'static,
{
    let (tx, rx) = mpsc::channel(5);
    tokio::spawn(async move {
        for routewkb in routewkbs {
            if let Err(e) = tx.send(Ok(routewkb)).await {
                warn!("Streaming routes aborted. reason: {}", e);
                break;
            }
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}

pub fn change_cell_resolution_dedup(
    cells: &[CellIndex],
    h3_resolution: Resolution,
) -> Vec<CellIndex> {
    let mut out_cells: Vec<_> = transform_resolution(cells, h3_resolution).collect();
    out_cells.sort_unstable();
    out_cells.dedup();
    out_cells
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
    debug!(
        "responding with a dataframe {:?} as a stream of chunks (max rows = {})",
        df_shape, max_rows
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
        for mut df_part in dataframe_parts {
            let serialization_result = block_in_place(|| dataframe_to_bytes(&mut df_part))
                .to_status_result_with_message(Code::Internal, || {
                    "serializing dataframe failed".to_string()
                })
                .map(|ipc_bytes| ArrowIpcChunk {
                    object_id: id.clone(),
                    data: ipc_bytes,
                });
            if let Err(e) = tx.send(serialization_result).await {
                warn!("Streaming dataframe parts aborted. reason: {}", e);
                break;
            }
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}

/// serialize a [`DataFrame`] into arrow IPC format
fn dataframe_to_bytes(dataframe: &mut DataFrame) -> Result<Vec<u8>, Status> {
    let mut buf: Vec<u8> = Vec::with_capacity(30_000);
    IpcWriter::new(&mut buf)
        .finish(dataframe)
        .to_status_result_with_message(Code::Internal, || {
            "serializing dataframe to Arrow IPC failed".to_string()
        })?;
    Ok(buf)
}

/// add a prefix to all columns in the dataframe
pub fn prefix_column_names(dataframe: &mut DataFrame, prefix: &str) -> Result<(), Status> {
    let col_names = dataframe
        .get_column_names()
        .into_iter()
        .map(|cn| cn.to_string())
        .sorted_by_key(|cn| cn.len()) // sort by length descending to avoid duplicated column names -> error
        .rev()
        .collect::<Vec<_>>();

    for col_name in col_names {
        dataframe
            .rename(&col_name, &format!("{prefix}{col_name}"))
            .to_status_result_with_message(Code::Internal, || {
                format!("prefixing column {col_name} with {prefix} failed")
            })?;
    }
    Ok(())
}

/// inner-join a [`CellDataFrame`] to the given `dataframe` using the specified `prefix`
pub fn inner_join_h3dataframe(
    dataframe: &mut DataFrame,
    dataframe_h3index_column: &str,
    mut celldataframe: CellDataFrame,
    prefix: &str,
) -> Result<(), Status> {
    // add prefix for origin columns
    prefix_column_names(&mut celldataframe.dataframe, prefix)?;

    *dataframe = dataframe
        .join(
            &celldataframe.dataframe,
            [dataframe_h3index_column],
            [format!("{}{}", prefix, celldataframe.cell_column_name.as_str()).as_str()],
            JoinArgs::new(JoinType::Inner),
        )
        .to_status_result_with_message(Code::Internal, || {
            "joining polars dataframes failed".to_string()
        })?;
    Ok(())
}
