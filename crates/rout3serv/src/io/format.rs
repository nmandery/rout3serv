use std::io::Cursor;
use std::path::Path;

use polars::prelude::{DataFrame, IpcReader, ParquetReader, SerReader};

use crate::io::Error;

#[derive(PartialEq, Eq, Debug)]
pub enum FileFormat {
    ArrowIPC,
    Parquet,
}

impl FileFormat {
    pub fn from_filename(filename: &str) -> Result<Self, Error> {
        let normalized_filename = filename.trim().to_lowercase();
        let path = Path::new(normalized_filename.as_str());
        match path.extension().and_then(|os| os.to_str()) {
            Some("arrow") => Ok(Self::ArrowIPC),
            Some("parquet") | Some("pq") => Ok(Self::Parquet),
            _ => Err(Error::UnidentifiedFileFormat(filename.to_string())),
        }
    }

    pub fn dataframe_from_slice(&self, bytes: &[u8]) -> Result<DataFrame, Error> {
        let cursor = Cursor::new(bytes);
        match self {
            FileFormat::ArrowIPC => Ok(IpcReader::new(cursor).finish()?),
            FileFormat::Parquet => Ok(ParquetReader::new(cursor).finish()?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileFormat;

    #[test]
    fn fileformat_from_filename() {
        assert_eq!(
            FileFormat::from_filename("/foo/bar.arrow").unwrap(),
            FileFormat::ArrowIPC
        );
        assert_eq!(
            FileFormat::from_filename("/foo/bar.parquet").unwrap(),
            FileFormat::Parquet
        );
        assert!(FileFormat::from_filename("/foo/bar.tiff").is_err());
        assert!(FileFormat::from_filename("/foo/bar").is_err());
    }
}
