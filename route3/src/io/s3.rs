use eyre::Result;
use futures::TryStreamExt;
use rusoto_core::credential::{AwsCredentials, StaticProvider};
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, S3};
use serde::Deserialize;
use std::str::FromStr;

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

    pub async fn get_object(&self, bucket: String, key: String) -> Result<Vec<u8>> {
        let get_object_req = GetObjectRequest {
            bucket,
            key,
            ..Default::default()
        };
        // TODO: explicitly handle 404
        let mut object = self.s3.get_object(get_object_req).await?;
        if let Some(body_stream) = object.body.take() {
            Ok(body_stream.map_ok(|b| b.to_vec()).try_concat().await?)
        } else {
            Ok(vec![]) // has no body
        }
    }
}
