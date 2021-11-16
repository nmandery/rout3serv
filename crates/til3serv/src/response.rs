use std::convert::Infallible;
use std::io::Cursor;

use axum::body::{Bytes, Full};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use polars_core::prelude::DataFrame;
use polars_io::SerWriter;
use serde::Serialize;

#[derive(Copy, Clone)]
pub enum OutputFormat {
    Json,
    ArrowIPC,
    Parquet,
    Csv,
}

impl OutputFormat {
    pub fn from_name(name: &str) -> Result<Self, StatusCode> {
        match name.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "arrow" | "ipc" => Ok(Self::ArrowIPC),
            "parquet" | "pq" => Ok(Self::Parquet),
            "csv" => Ok(Self::Csv),
            _ => {
                log::warn!("unknown format: {}", name);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            OutputFormat::Json => "application/jsonlines+json",
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

#[derive(Serialize)]
pub struct Msg {
    pub message: String,
}

impl Msg {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Default for Msg {
    fn default() -> Self {
        Self {
            message: "something went wrong :(".to_string(),
        }
    }
}

pub struct OutDataFrame {
    pub output_format: OutputFormat,
    pub h3_resolution: u8,
    pub dataframe: DataFrame,
}

impl IntoResponse for OutDataFrame {
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        log::debug!(
            "responding with dataframe with shape {:?}",
            self.dataframe.shape()
        );
        match outdf_to_response(self) {
            Ok(response) => response,
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(axum::http::header::CONTENT_TYPE, "text/plain")
                .body(Full::from(err.to_string()))
                .unwrap(),
        }
    }
}

fn outdf_to_response(outdf: OutDataFrame) -> eyre::Result<Response<Full<Bytes>>> {
    let mut bytes = vec![];
    let status = if outdf.dataframe.is_empty() {
        StatusCode::NO_CONTENT
    } else {
        match &outdf.output_format {
            OutputFormat::Json => {
                let writer = polars_io::json::JsonWriter::new(&mut bytes);
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
        .body(bytes.into())
        .unwrap())
}
