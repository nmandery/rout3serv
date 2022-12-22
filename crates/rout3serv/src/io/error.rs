use tokio::task::JoinError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),

    #[error("not a graph key")]
    NotAGraphKey,

    #[error("deserialize panic")]
    DeserializePanic,

    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Polars(#[from] polars::prelude::PolarsError),

    #[error("unidentified file format: {0}")]
    UnidentifiedFileFormat(String),

    #[error("unsupported h3 resolution: {0}")]
    UnsupportedH3Resolution(u8),

    #[error("join error")]
    Join,

    #[error(transparent)]
    H3ron(#[from] h3ron::Error),

    #[error(transparent)]
    H3ronPolars(#[from] h3ron_polars::Error),
}

impl From<tokio::task::JoinError> for Error {
    fn from(_: JoinError) -> Self {
        Self::Join
    }
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::ObjectStore(object_store::Error::NotFound { .. })
                | Self::UnsupportedH3Resolution(_)
        )
    }
}
