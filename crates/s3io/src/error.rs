use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("h3ron error: {0}")]
    H3ron(#[from] h3ron::Error),

    #[error("polars error: {0}")]
    Polars(#[from] polars_core::error::PolarsError),

    #[error("unidentified file format: {0}")]
    UnidentifiedFileFormat(String),

    #[error("dataframe h3index column '{0}' is typed as {1}, but should be UInt64")]
    DataframeInvalidH3IndexType(String, String),

    #[error("dataframe contains no column named '{0}'")]
    DataframeMissingColumn(String),

    #[error("Invalid S3 region: {0}")]
    InvalidS3Region(#[from] rusoto_core::region::ParseRegionError),

    #[error("listing S3 objects failed: {0}")]
    S3ListObjects(#[from] rusoto_core::RusotoError<rusoto_s3::ListObjectsError>),

    #[error("GETting S3 object failed: {0}")]
    S3GetObject(#[from] rusoto_core::RusotoError<rusoto_s3::GetObjectError>),

    #[error("PUTing S3 object failed: {0}")]
    S3PutObject(#[from] rusoto_core::RusotoError<rusoto_s3::PutObjectError>),

    #[error("S3 TLS error: {0}")]
    S3TLS(#[from] rusoto_core::request::TlsError),

    #[error("native tls error: {0}")]
    NativeTLS(#[from] native_tls::Error),

    #[error("tokio join error: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Unsupported H3 resolution: {0}")]
    UnsupportedH3Resolution(u8),

    #[error("not found")]
    NotFound,

    #[error("{0}")]
    Generic(String),
}
