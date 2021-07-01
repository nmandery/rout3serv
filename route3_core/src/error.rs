use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("h3ron error: {0}")]
    H3ronError(#[from] h3ron::Error),

    #[cfg(feature = "with-gdal")]
    #[error("gdal error: {0}")]
    GdalError(#[from] gdal::errors::GdalError),

    #[error("bincode error: {0}")]
    BincodeError(#[from] bincode::Error),

    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("json error: {0}")]
    JSONError(#[from] serde_json::Error),
}
