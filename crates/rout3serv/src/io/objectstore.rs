use std::ops::Deref;

use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::RetryConfig;
use serde::Deserialize;

use crate::io::Error;

pub struct ObjectStore(pub Box<dyn object_store::ObjectStore>);

impl Deref for ObjectStore {
    type Target = dyn object_store::ObjectStore;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ObjectStoreConfig {
    #[serde(alias = "filesystem")]
    Filesystem { root: String },

    /// S3 configured by environment variables
    #[serde(alias = "s3-by-env")]
    S3ByEnvironment {},

    #[serde(alias = "s3")]
    S3 {
        endpoint: String,
        access_key: String,
        secret_access_key: String,
        region: String,
        bucket_name: String,
        allow_http: Option<bool>,
    },
}

impl TryFrom<ObjectStoreConfig> for ObjectStore {
    type Error = Error;

    fn try_from(sc: ObjectStoreConfig) -> Result<Self, Self::Error> {
        let store = match sc {
            ObjectStoreConfig::Filesystem { root } => {
                Self(Box::new(LocalFileSystem::new_with_prefix(root)?))
            }
            ObjectStoreConfig::S3ByEnvironment {} => {
                let builder = AmazonS3Builder::from_env().with_retry(RetryConfig::default());
                Self(Box::new(builder.build()?))
            }
            ObjectStoreConfig::S3 {
                endpoint,
                access_key,
                secret_access_key,
                region,
                bucket_name,
                allow_http,
            } => {
                let builder = AmazonS3Builder::new()
                    .with_endpoint(endpoint)
                    .with_region(region)
                    .with_access_key_id(access_key)
                    .with_secret_access_key(secret_access_key)
                    .with_allow_http(allow_http.unwrap_or(false))
                    .with_bucket_name(bucket_name)
                    .with_retry(RetryConfig::default());

                Self(Box::new(builder.build()?))
            }
        };
        Ok(store)
    }
}
