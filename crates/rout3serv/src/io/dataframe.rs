use std::collections::HashMap;

use serde::Deserialize;
use tracing::error;

use crate::io::format::FileFormat;
use crate::io::Error;

#[derive(Deserialize)]
pub struct DataframeDataset {
    pub key_pattern: String,
    /// maps data resolutions to the file h3 resolutions
    pub resolutions: HashMap<u8, u8>,

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

    pub fn file_h3_resolution(&self, data_h3_resolution: u8) -> Result<u8, Error> {
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
