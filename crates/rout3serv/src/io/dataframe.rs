use h3o::Resolution;
use polars_core::prelude::{DataFrame, UInt64Chunked};
use std::collections::HashMap;

use serde::Deserialize;
use tracing::error;

use crate::io::format::FileFormat;
use crate::io::Error;
use crate::io::Error::MissingCellColumn;

#[derive(Deserialize)]
pub struct DataframeDataset {
    pub key_pattern: String,
    /// maps data resolutions to the file h3 resolutions
    pub resolutions: HashMap<Resolution, Resolution>,

    pub h3index_column_name: String,
}

impl DataframeDataset {
    pub fn fileformat(&self) -> Result<FileFormat, Error> {
        FileFormat::from_filename(&self.key_pattern)
    }

    pub fn validate(&self) -> Result<(), Error> {
        // try to check if the format is understood
        self.fileformat()?;
        Ok(())
    }

    pub fn file_h3_resolution(&self, data_h3_resolution: Resolution) -> Result<Resolution, Error> {
        self.resolutions
            .get(&data_h3_resolution)
            .copied()
            .ok_or_else(|| {
                error!(
                    "unsupported h3 resolution for building a key: {}",
                    data_h3_resolution
                );
                Error::UnsupportedH3Resolution(data_h3_resolution)
            })
    }
}

pub trait ToDataFrame {
    fn to_dataframe(&self) -> Result<DataFrame, Error>;
}

pub trait FromDataFrame {
    fn from_dataframe(df: DataFrame) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CellDataFrame {
    pub dataframe: DataFrame,
    pub cell_column_name: String,
}

impl CellDataFrame {
    pub fn cell_u64s(&self) -> Result<&UInt64Chunked, Error> {
        self.dataframe
            .column(&self.cell_column_name)
            .map_err(|_| MissingCellColumn(self.cell_column_name.clone()))?
            .u64()
            .map_err(Error::from)
    }
}
