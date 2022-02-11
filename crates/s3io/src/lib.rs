#![warn(
    clippy::all,
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    nonstandard_style
)]

#[macro_use]
extern crate lazy_static;

pub use crate::error::Error;

pub mod dataframe;
mod error;
pub mod fetch;
pub mod format;
pub mod s3;
pub mod ser_and_de;
