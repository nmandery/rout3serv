use h3o::Resolution;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("decompression error")]
    DecompressionError(String),

    #[error(transparent)]
    OutlinerError(#[from] h3o::error::OutlinerError),

    #[error(transparent)]
    InvalidDirectedEdgeIndex(#[from] h3o::error::InvalidDirectedEdgeIndex),

    #[error(transparent)]
    InvalidGeometry(#[from] h3o::error::InvalidGeometry),

    #[error("too high h3 resolution: {0}")]
    TooHighH3Resolution(Resolution),

    #[error("mixed h3 resolutions: {0} <> {1}")]
    MixedH3Resolutions(Resolution, Resolution),

    #[error("empty path")]
    EmptyPath,

    #[error("insufficient number of edges")]
    InsufficientNumberOfEdges,

    #[error("path is segmented into multiple parts")]
    SegmentedPath,

    #[error("none of the routing destinations is part of the routing graph")]
    DestinationsNotInGraph,

    #[error("empty exclude cells")]
    EmptyExcludeCells,

    #[error("minimum fastforward length must be >= {0}")]
    TooShortLongEdge(usize),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}
