use std::collections::HashSet;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use arrow::record_batch::RecordBatch;
use bytes::Bytes;
use bytesize::ByteSize;
use futures::TryStreamExt;
use h3ron::iter::change_cell_resolution;
use h3ron::H3Cell;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use native_tls::TlsConnector;
use regex::Regex;
use rusoto_core::credential::{AwsCredentials, StaticProvider};
use rusoto_core::{ByteStream, HttpClient, Region, RusotoError};
use rusoto_s3::{GetObjectRequest, ListObjectsRequest, PutObjectRequest, S3};
use serde::Deserialize;
use tokio::task;

use crate::dataframe::H3DataFrame;
use crate::format::FileFormat;
use crate::Error;

#[derive(Deserialize)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub insecure: Option<bool>,
    pub region: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub retry_seconds: Option<u64>,
}

fn env_override<K: AsRef<str>, F: FnOnce() -> String>(key: K, default_value: F) -> String {
    match env::var(key.as_ref()) {
        Ok(value) => {
            log::info!("Using override from environment variable {}", key.as_ref());
            value
        }
        Err(_) => default_value(),
    }
}

impl S3Config {
    /// get the s3 access key - may be overridden using the `S3_ACCESS_KEY`
    /// environment variable
    pub fn get_access_key(&self) -> String {
        env_override("S3_ACCESS_KEY", || self.access_key.clone())
    }

    /// get the s3 secret key - may be overridden using the `S3_SECRET_KEY`
    /// environment variable
    pub fn get_secret_key(&self) -> String {
        env_override("S3_SECRET_KEY", || self.secret_key.clone())
    }
}

/// reference to a S3 object
#[derive(PartialOrd, PartialEq, Clone, Hash, Eq)]
pub struct ObjectRef {
    pub bucket: String,
    pub key: String,
}

impl ObjectRef {
    pub fn new(bucket: String, key: String) -> Self {
        Self { bucket, key }
    }
}

impl std::fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectRef(bucket={}, key={})", self.bucket, self.key)
    }
}

pub struct S3Client {
    s3: rusoto_s3::S3Client,
    retry_duration: Duration,
}

impl S3Client {
    pub fn from_config(config: &S3Config) -> Result<Self, Error> {
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
                build_http_client(config.insecure.unwrap_or(false))?,
                StaticProvider::from(AwsCredentials::new(
                    config.get_access_key(),
                    config.get_secret_key(),
                    None,
                    None,
                )),
                region,
            ),
            retry_duration: Duration::from_secs(config.retry_seconds.unwrap_or(20)),
        })
    }

    pub async fn get_object_bytes(&self, object_ref: ObjectRef) -> Result<Vec<u8>, Error> {
        log::debug!("get_object_bytes: {}", object_ref);
        let ob = backoff::future::retry(
            backoff::ExponentialBackoff {
                max_elapsed_time: Some(self.retry_duration),
                ..Default::default()
            },
            || async {
                let get_object_req = GetObjectRequest {
                    bucket: object_ref.bucket.clone(),
                    key: object_ref.key.clone(),
                    ..Default::default()
                };
                Ok(match self.s3.get_object(get_object_req).await {
                    Ok(mut object) => {
                        if let Some(body_stream) = object.body.take() {
                            let byte_content: Vec<u8> = body_stream
                                .map_ok(|b| b.to_vec())
                                .try_concat()
                                .await
                                .map_err(Error::from)?;
                            log::info!(
                                "get_object_bytes: {} -> received {} bytes ({})",
                                object_ref,
                                byte_content.len(),
                                ByteSize(byte_content.len() as u64)
                            );
                            Ok(byte_content)
                        } else {
                            Ok(vec![]) // has no body
                        }
                    }
                    Err(e) => match e {
                        RusotoError::Service(_get_object_error) => {
                            log::warn!("get_object_bytes: {} -> not found", object_ref);
                            Err(Error::NotFound)
                        }
                        _ => {
                            log::error!("get_object_bytes: {} -> {}", object_ref, e.to_string());
                            Err(Error::S3GetObject(e))
                        }
                    },
                }?)
            },
        )
        .await?;
        Ok(ob)
    }

    pub async fn put_object_bytes(
        &self,
        object_ref: ObjectRef,
        data: Vec<u8>,
    ) -> Result<(), Error> {
        log::info!("put_object_bytes: {}, num_bytes={}", object_ref, data.len());

        let data_bytes = Bytes::from(data);

        let ob = backoff::future::retry(
            backoff::ExponentialBackoff {
                max_elapsed_time: Some(self.retry_duration),
                ..Default::default()
            },
            || async {
                let data_bytes_this_try = data_bytes.clone();
                let byte_stream = ByteStream::new_with_size(
                    futures::stream::once(async move { Ok(data_bytes_this_try) }),
                    data_bytes.len(),
                );

                let put_object_req = PutObjectRequest {
                    bucket: object_ref.bucket.clone(),
                    key: object_ref.key.clone(),
                    body: Some(byte_stream),
                    ..Default::default()
                };
                match self.s3.put_object(put_object_req).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("put_object_bytes: {} -> {}", object_ref, e.to_string());
                        Err(e.into())
                    }
                }
            },
        )
        .await?;
        Ok(ob)
    }

    pub async fn list_object_keys(
        &self,
        bucket: String,
        prefix: Option<String>,
    ) -> Result<Vec<String>, Error> {
        let list_req = ListObjectsRequest {
            bucket: bucket.clone(),
            delimiter: None,
            encoding_type: None,
            expected_bucket_owner: None,
            marker: None,
            max_keys: Some(600),
            prefix: prefix.clone(),
            request_payer: None,
        };
        match self.s3.list_objects(list_req).await {
            Ok(lo_output) => Ok(lo_output
                .contents
                .map(|mut objects| objects.drain(..).filter_map(|object| object.key).collect())
                .unwrap_or_else(Vec::new)),
            Err(e) => {
                log::error!(
                    "list_object_keys: bucket={}, key={} -> {}",
                    bucket,
                    prefix.unwrap_or_default(),
                    e.to_string()
                );
                Err(e.into())
            }
        }
    }
}

fn build_http_client(insecure: bool) -> Result<HttpClient, Error> {
    let http_client = if insecure {
        // from https://rusoto.org/disable-ssl-cert-check.html
        let tls_connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let mut http_connector = HttpConnector::new();
        http_connector.enforce_http(false);

        let https_connector = HttpsConnector::from((http_connector, tls_connector.into()));
        HttpClient::from_connector(https_connector)
    } else {
        HttpClient::new()?
    };
    Ok(http_client)
}

pub trait S3H3Dataset {
    fn bucket_name(&self) -> String;
    fn key_pattern(&self) -> String;
    fn h3index_column(&self) -> String;

    fn validate(&self) -> Result<(), Error> {
        // try to check if the format is understood
        FileFormat::from_filename(&self.key_pattern())?;
        Ok(())
    }

    fn file_h3_resolution(&self, data_h3_resolution: u8) -> Option<u8>;

    fn file_h3_resolution_checked(&self, data_h3_resolution: u8) -> Result<u8, Error> {
        let fr = self.file_h3_resolution(data_h3_resolution).ok_or_else(|| {
            log::error!(
                "unsupported h3 resolution for building a key: {}",
                data_h3_resolution
            );
            Error::UnsupportedH3Resolution(data_h3_resolution)
        })?;
        Ok(fr)
    }
}

lazy_static! {
    static ref RE_S3KEY_DATA_H3_RESOLUTION: Regex =
        Regex::new(r"\{\s*data_h3_resolution\s*\}").unwrap();
    static ref RE_S3KEY_FILE_H3_RESOLUTION: Regex =
        Regex::new(r"\{\s*file_h3_resolution\s*\}").unwrap();
    static ref RE_S3KEY_H3_CELL: Regex = Regex::new(r"\{\s*h3cell\s*\}").unwrap();
}

fn build_h3_key<D>(dataset: &D, cell: &H3Cell, data_h3_resolution: u8) -> Result<String, Error>
where
    D: S3H3Dataset,
{
    Ok(RE_S3KEY_H3_CELL
        .replace_all(
            &RE_S3KEY_FILE_H3_RESOLUTION.replace_all(
                &RE_S3KEY_DATA_H3_RESOLUTION.replace_all(
                    dataset.key_pattern().as_ref(),
                    data_h3_resolution.to_string(),
                ),
                dataset
                    .file_h3_resolution_checked(data_h3_resolution)
                    .map(|r| r.to_string())?,
            ),
            cell.to_string(),
        )
        .to_string())
}

pub struct S3RecordBatchLoader {
    s3_client: Arc<S3Client>,
}

impl S3RecordBatchLoader {
    pub fn new(s3_client: Arc<S3Client>) -> Self {
        Self { s3_client }
    }

    async fn load_h3_dataset_recordbatches<D: S3H3Dataset>(
        &self,
        dataset: &D,
        cells: &[H3Cell],
        data_h3_resolution: u8,
    ) -> Result<Vec<RecordBatch>, Error> {
        if cells.is_empty() {
            return Ok(Default::default());
        }
        let file_cells = change_cell_resolution(
            cells.iter(),
            dataset.file_h3_resolution_checked(data_h3_resolution)?,
        )
        .collect::<HashSet<_>>();

        let mut keys = file_cells
            .iter()
            .map(|cell| build_h3_key(dataset, cell, data_h3_resolution))
            .collect::<Result<Vec<_>, _>>()?;
        keys.sort_unstable(); // remove duplicates when the keys are not grouped using a file resolution
        keys.dedup();

        let mut task_results = futures::future::try_join_all(keys.drain(..).map(|key| {
            let object_ref = ObjectRef::new(dataset.bucket_name(), key);
            let s3_client = self.s3_client.clone();
            task::spawn(async move {
                let format = FileFormat::from_filename(&object_ref.key)?;
                match s3_client.get_object_bytes(object_ref).await {
                    Ok(bytes) => Ok(format.recordbatches_from_slice(&bytes)?),
                    Err(Error::NotFound) => Ok(vec![]),
                    Err(e) => Err(e),
                }
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

    pub async fn load_h3_dataset_dataframe<D: S3H3Dataset>(
        &self,
        dataset: &D,
        cells: &[H3Cell],
        data_h3_resolution: u8,
    ) -> Result<H3DataFrame, Error> {
        H3DataFrame::from_recordbatches(
            self.load_h3_dataset_recordbatches(dataset, cells, data_h3_resolution)
                .await?,
            dataset.h3index_column(),
        )
    }
}
