#[macro_use]
extern crate lazy_static;

pub use crate::error::Error;

pub mod dataframe;
mod error;
pub mod format;
pub mod s3;
