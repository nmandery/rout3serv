use std::io::Cursor;
use std::path::Path;

use arrow::io::{ipc, parquet};
use arrow::record_batch::RecordBatch;
use eyre::{Report, Result};

#[derive(PartialEq, Debug)]
pub enum FileFormat {
    ArrowIPC,
    Parquet,
}

impl FileFormat {
    pub fn from_filename(filename: &str) -> Result<Self> {
        let normalized_filename = filename.trim().to_lowercase();
        let path = Path::new(normalized_filename.as_str());
        match path.extension().map(|os| os.to_str()).flatten() {
            Some("arrow") => Ok(Self::ArrowIPC),
            Some("parquet") | Some("pq") => Ok(Self::Parquet),
            _ => Err(Report::msg(format!(
                "unidentified fileformat: {}",
                filename
            ))),
        }
    }

    pub fn recordbatches_from_slice(&self, bytes: &[u8]) -> Result<Vec<RecordBatch>> {
        let mut recordbatches = vec![];
        let mut cursor = Cursor::new(bytes);
        match self {
            FileFormat::ArrowIPC => {
                let metadata = ipc::read::read_file_metadata(&mut cursor)?;
                for recordbatch in ipc::read::FileReader::new(&mut cursor, metadata, None) {
                    recordbatches.push(recordbatch?);
                }
            }
            FileFormat::Parquet => {
                for recordbatch in
                    parquet::read::RecordReader::try_new(&mut cursor, None, None, None, None)?
                {
                    recordbatches.push(recordbatch?);
                }
            }
        };
        Ok(recordbatches)
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
