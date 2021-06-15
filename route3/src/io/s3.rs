use std::str::FromStr;

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
}

pub struct S3Client {
    s3: rusoto_s3::S3Client,
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
                endpoint: endpoint.clone(),
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
        })
    }

    pub async fn get_object_bytes<S: AsRef<str>>(&self, bucket: S, key: S) -> Result<ObjectBytes> {
        log::info!(
            "get_object_bytes: bucket={}, key={}",
            bucket.as_ref(),
            key.as_ref()
        );
        let get_object_req = GetObjectRequest {
            bucket: bucket.as_ref().to_owned(),
            key: key.as_ref().to_owned(),
            ..Default::default()
        };
        match self.s3.get_object(get_object_req).await {
            Ok(mut object) => {
                if let Some(body_stream) = object.body.take() {
                    let byte_content: Vec<u8> =
                        body_stream.map_ok(|b| b.to_vec()).try_concat().await?;
                    log::info!(
                        "get_object_bytes: bucket={}, key={} -> received {} bytes ({})",
                        bucket.as_ref(),
                        key.as_ref(),
                        byte_content.len(),
                        ByteSize(byte_content.len() as u64)
                    );
                    Ok(ObjectBytes::Found(byte_content))
                } else {
                    Ok(ObjectBytes::Found(vec![])) // has no body
                }
            }
            Err(e) => match e {
                RusotoError::Service(_get_object_error) => Ok(ObjectBytes::NotFound),
                _ => Err(Report::from(e)),
            },
        }
    }
}
