use h3o::Resolution;
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
    UnsupportedH3Resolution(Resolution),

    #[error("join error")]
    Join,

    #[error("missing cell column {0}")]
    MissingCellColumn(String),

    #[error(transparent)]
    InvalidDirectedEdgeIndex(#[from] h3o::error::InvalidDirectedEdgeIndex),

    #[error(transparent)]
    InvalidCellIndex(#[from] h3o::error::InvalidCellIndex),

    #[error(transparent)]
    Hexigraph(#[from] hexigraph::error::Error),
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
