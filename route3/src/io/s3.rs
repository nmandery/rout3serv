use std::collections::HashSet;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use arrow::ipc::reader::FileReader;
use arrow::record_batch::RecordBatch;
use bytesize::ByteSize;
use eyre::{Report, Result};
use futures::TryStreamExt;
use regex::Regex;
use rusoto_core::credential::{AwsCredentials, StaticProvider};
use rusoto_core::{Region, RusotoError};
use rusoto_s3::{GetObjectRequest, S3};
use serde::Deserialize;
use tokio::task;

use route3_core::algo::iter::change_h3_resolution;
use route3_core::h3ron::H3Cell;

#[derive(Deserialize)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub retry_seconds: Option<u64>,
}

pub struct S3Client {
    s3: rusoto_s3::S3Client,
    retry_duration: Duration,
}

pub enum ObjectBytes {
    Found(Vec<u8>),
    NotFound,
}

impl S3Client {
    pub fn from_config(config: &S3Config) -> Result<Self> {
        let region_string = config
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_owned());
        let region = if let Some(endpoint) = &config.endpoint {
            Region::Custom {
                name: region_string,
                endpoint: endpoint.trim_end_matches('/').to_owned(),
            }
        } else {
            Region::from_str(&region_string)?
        };

        Ok(Self {
            s3: rusoto_s3::S3Client::new_with(
                rusoto_core::request::HttpClient::new()?,
                StaticProvider::from(AwsCredentials::new(
                    config.access_key.clone(),
                    config.secret_key.clone(),
                    None,
                    None,
                )),
                region,
            ),
            retry_duration: Duration::from_secs(config.retry_seconds.unwrap_or(20)),
        })
    }

    pub async fn get_object_bytes(&self, bucket: String, key: String) -> Result<ObjectBytes> {
        log::debug!("get_object_bytes: bucket={}, key={}", bucket, key);
        let ob = backoff::future::retry(
            backoff::ExponentialBackoff {
                max_elapsed_time: Some(self.retry_duration),
                ..Default::default()
            },
            || async {
                let get_object_req = GetObjectRequest {
                    bucket: bucket.clone(),
                    key: key.clone(),
                    ..Default::default()
                };
                Ok(match self.s3.get_object(get_object_req).await {
                    Ok(mut object) => {
                        if let Some(body_stream) = object.body.take() {
                            let byte_content: Vec<u8> = body_stream
                                .map_ok(|b| b.to_vec())
                                .try_concat()
                                .await
                                .map_err(Report::from)?;
                            log::info!(
                                "get_object_bytes: bucket={}, key={} -> received {} bytes ({})",
                                bucket,
                                key,
                                byte_content.len(),
                                ByteSize(byte_content.len() as u64)
                            );
                            Ok(ObjectBytes::Found(byte_content))
                        } else {
                            Ok(ObjectBytes::Found(vec![])) // has no body
                        }
                    }
                    Err(e) => match e {
                        RusotoError::Service(_get_object_error) => {
                            log::info!(
                                "get_object_bytes: bucket={}, key={} -> not found",
                                bucket,
                                key
                            );
                            Ok(ObjectBytes::NotFound)
                        }
                        _ => {
                            log::error!(
                                "get_object_bytes: bucket={}, key={} -> {}",
                                bucket,
                                key,
                                e.to_string()
                            );
                            Err(Report::from(e))
                        }
                    },
                }?)
            },
        )
        .await?;
        Ok(ob)
    }
}

pub trait S3H3Dataset {
    fn bucket_name(&self) -> String;
    fn key_pattern(&self) -> String;
    fn file_h3_resolution(&self) -> u8;
}

lazy_static! {
    static ref RE_S3KEY_DATA_H3_RESOLUTION: Regex =
        Regex::new(r"\{\s*data_h3_resolution\s*\}").unwrap();
    static ref RE_S3KEY_FILE_H3_RESOLUTION: Regex =
        Regex::new(r"\{\s*file_h3_resolution\s*\}").unwrap();
    static ref RE_S3KEY_H3_CELL: Regex = Regex::new(r"\{\s*h3cell\s*\}").unwrap();
}

pub struct S3RecordBatchLoader {
    s3_client: Arc<S3Client>,
}

impl S3RecordBatchLoader {
    pub fn new(s3_client: Arc<S3Client>) -> Self {
        Self { s3_client }
    }

    pub async fn load_h3_dataset<D: S3H3Dataset>(
        &self,
        dataset: D,
        cells: &[H3Cell],
        data_h3_resolution: u8,
    ) -> Result<Vec<RecordBatch>> {
        if cells.is_empty() {
            return Ok(Default::default());
        }
        let file_cells = change_h3_resolution(cells.iter(), dataset.file_h3_resolution())
            .collect::<HashSet<_>>();

        let mut task_results = futures::future::try_join_all(file_cells.iter().map(|cell| {
            let bucket_name = dataset.bucket_name();
            let key = self.build_h3_key(&dataset, cell, data_h3_resolution);
            let s3_client = self.s3_client.clone();
            task::spawn(async move {
                s3_client
                    .get_object_bytes(bucket_name, key)
                    .await
                    .and_then(|object_bytes| {
                        let mut record_batches = vec![];
                        if let ObjectBytes::Found(bytes) = object_bytes {
                            let cursor = Cursor::new(&bytes);
                            for record_batch in FileReader::try_new(cursor)? {
                                record_batches.push(record_batch?);
                            }
                        };
                        Ok(record_batches)
                    })
            })
        }))
        .await?;

        let mut record_batches = Vec::with_capacity(file_cells.len());
        for task_result in task_results.drain(..) {
            for rb in task_result?.drain(..) {
                record_batches.push(rb);
            }
        }
        Ok(record_batches)
    }

    fn build_h3_key<D: S3H3Dataset>(
        &self,
        dataset: &D,
        cell: &H3Cell,
        data_h3_resolution: u8,
    ) -> String {
        RE_S3KEY_H3_CELL
            .replace_all(
                &RE_S3KEY_FILE_H3_RESOLUTION.replace_all(
                    &RE_S3KEY_DATA_H3_RESOLUTION.replace_all(
                        dataset.key_pattern().as_ref(),
                        data_h3_resolution.to_string(),
                    ),
                    dataset.file_h3_resolution().to_string(),
                ),
                cell.to_string(),
            )
            .to_string()
    }
}
