#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Geos(#[from] geos::Error),

    #[error(transparent)]
    Geozero(#[from] geozero::error::GeozeroError),
}
