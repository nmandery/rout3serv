use crate::io::dataframe::{FromDataFrame, ToDataFrame};
use crate::io::Error;
use polars::io::mmap::MmapBytesReader;
use polars::prelude::{ParquetCompression, ParquetReader, ParquetWriter, SerReader};
use std::io::Write;

fn write_parquet<Writer, T>(writer: Writer, value: &T) -> Result<(), Error>
where
    Writer: Write,
    T: ToDataFrame,
{
    let mut df = value.to_dataframe()?;
    let pqw = ParquetWriter::new(writer).with_compression(ParquetCompression::Zstd(None));
    pqw.finish(&mut df)?;
    Ok(())
}

fn read_parquet<Reader: MmapBytesReader, T>(reader: Reader) -> Result<T, Error>
where
    T: FromDataFrame,
{
    T::from_dataframe(ParquetReader::new(reader).finish()?)
}

pub trait WriteParquet {
    fn write_parquet<Writer>(&self, writer: Writer) -> Result<(), Error>
    where
        Writer: Write;
}

impl<T> WriteParquet for T
where
    T: ToDataFrame,
{
    fn write_parquet<Writer>(&self, writer: Writer) -> Result<(), Error>
    where
        Writer: Write,
    {
        write_parquet(writer, self)
    }
}

pub trait ReadParquet {
    fn read_parquet<Reader: MmapBytesReader>(reader: Reader) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<T> ReadParquet for T
where
    T: FromDataFrame,
{
    fn read_parquet<Reader: MmapBytesReader>(reader: Reader) -> Result<Self, Error>
    where
        Self: Sized,
    {
        read_parquet(reader)
    }
}
