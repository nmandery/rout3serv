use std::str::FromStr;
use std::time::Duration;

use bytesize::ByteSize;
use eyre::{Report, Result};
use futures::TryStreamExt;
use rusoto_core::credential::{AwsCredentials, StaticProvider};
use rusoto_core::{Region, RusotoError};
use rusoto_s3::{GetObjectRequest, S3};
use serde::Deserialize;

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
        log::info!("get_object_bytes: bucket={}, key={}", bucket, key);
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
                            log::error!("get_object_bytes: {}", e.to_string());
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
