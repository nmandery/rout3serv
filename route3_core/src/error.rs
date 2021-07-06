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

    #[error("mixed h3 resolutions: {0} <> {1}")]
    MixedH3Resolutions(u8, u8),

    #[error("too high h3 resolution: {0}")]
    TooHighH3Resolution(u8),

    #[error("empty route")]
    EmptyRoute,

    #[error("none of the routing destinatons is part of the routing graph")]
    DestinationsNotInGraph,

    #[cfg(feature = "osm")]
    #[error("osmpbfreader error: {0}")]
    OSMPbfReaderError(#[from] osmpbfreader::Error),
}
