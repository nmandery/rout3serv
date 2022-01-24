use axum::body;
use std::io::Cursor;

use axum::body::{BoxBody, Full};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use h3ron::{FromH3Index, H3Cell};
use polars_core::prelude::{DataFrame, Utf8Chunked};
use polars_io::json::JsonFormat;
use polars_io::SerWriter;

#[derive(Copy, Clone)]
pub enum OutputFormat {
    JsonLines,
    Json,
    ArrowIPC,
    Parquet,
    Csv,
}

impl OutputFormat {
    pub fn from_name(name: &str) -> Result<Self, StatusCode> {
        match name.to_lowercase().as_str() {
            "jl" | "jsonl" | "jsonlines" => Ok(Self::JsonLines),
            "j" | "json" => Ok(Self::Json),
            "arrow" | "ipc" => Ok(Self::ArrowIPC),
            "parquet" | "pq" => Ok(Self::Parquet),
            "csv" => Ok(Self::Csv),
            _ => {
                log::warn!("unknown format: {}", name);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }

    pub const fn content_type(&self) -> &'static str {
        match self {
            OutputFormat::JsonLines => "application/jsonlines+json",
            OutputFormat::Json => "application/json",
            OutputFormat::ArrowIPC => "application/vnd.apache.arrow.file",
            OutputFormat::Parquet => "application/parquet",
            OutputFormat::Csv => "text/csv",
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::ArrowIPC
    }
}

pub struct OutDataFrame {
    pub output_format: OutputFormat,
    pub h3_resolution: u8,
    pub dataframe: DataFrame,
}

impl OutDataFrame {
    pub const fn h3index_column_name() -> &'static str {
        "h3index"
    }
}

impl IntoResponse for OutDataFrame {
    fn into_response(self) -> Response {
        log::debug!(
            "responding with dataframe with shape {:?}",
            self.dataframe.shape()
        );
        match outdf_to_response(self) {
            Ok(response) => response,
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(axum::http::header::CONTENT_TYPE, "text/plain")
                .body(body::boxed(Full::from(err.to_string())))
                .unwrap(),
        }
    }
}

fn outdf_to_response(mut outdf: OutDataFrame) -> eyre::Result<Response<BoxBody>> {
    let mut bytes = vec![];
    let status = if outdf.dataframe.is_empty() {
        StatusCode::NO_CONTENT
    } else {
        // convert h3indexes to hex-strings as UInt64-support in browsers is still somewhat recent
        outdf.dataframe.replace_or_add(
            OutDataFrame::h3index_column_name(),
            outdf
                .dataframe
                .column(OutDataFrame::h3index_column_name())?
                .u64()?
                .into_iter()
                .map(|o| o.map(|h3index| H3Cell::from_h3index(h3index).to_string()))
                .collect::<Utf8Chunked>(),
        )?;

        match &outdf.output_format {
            OutputFormat::JsonLines => {
                let writer = polars_io::json::JsonWriter::with_json_format(
                    polars_io::json::JsonWriter::new(&mut bytes),
                    JsonFormat::JsonLines,
                );
                writer.finish(&outdf.dataframe)?;
            }
            OutputFormat::Json => {
                let writer = polars_io::json::JsonWriter::with_json_format(
                    polars_io::json::JsonWriter::new(&mut bytes),
                    JsonFormat::Json,
                );
                writer.finish(&outdf.dataframe)?;
            }
            OutputFormat::ArrowIPC => {
                let writer = polars_io::ipc::IpcWriter::new(&mut bytes);
                writer.finish(&outdf.dataframe)?;
            }
            OutputFormat::Parquet => {
                let cursor = Cursor::new(&mut bytes);
                let writer = polars_io::parquet::ParquetWriter::new(cursor);
                writer.finish(&outdf.dataframe)?;
            }
            OutputFormat::Csv => {
                let writer = polars_io::csv::CsvWriter::new(&mut bytes);
                writer.finish(&outdf.dataframe)?;
            }
        };

        StatusCode::OK
    };

    Ok(Response::builder()
        .status(status)
        .header(CONTENT_TYPE, outdf.output_format.content_type())
        .header("X-H3-Resolution", outdf.h3_resolution.to_string())
        .header("X-Shape", format!("{:?}", outdf.dataframe.shape()))
        .body(body::boxed(Full::from(bytes)))
        .unwrap())
}
