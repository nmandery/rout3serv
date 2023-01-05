use crate::io::dataframe::{FromDataFrame, ToDataFrame};
use crate::io::Error;
use polars::io::mmap::MmapBytesReader;
use polars::prelude::{IpcReader, IpcWriter, SerReader, SerWriter};
use polars_core::export::arrow::io::ipc::write::Compression;
use std::io::Write;

fn write_ipc<Writer, T>(writer: Writer, value: &T) -> Result<(), Error>
where
    Writer: Write,
    T: ToDataFrame,
{
    let mut df = value.to_dataframe()?;
    IpcWriter::new(writer)
        .with_compression(Some(Compression::ZSTD))
        .finish(&mut df)?;
    Ok(())
}

fn read_ipc<Reader: MmapBytesReader, T>(reader: Reader) -> Result<T, Error>
where
    T: FromDataFrame,
{
    T::from_dataframe(IpcReader::new(reader).finish()?)
}

pub trait WriteIPC {
    fn write_ipc<Writer>(&self, writer: Writer) -> Result<(), Error>
    where
        Writer: Write;
}

impl<T> WriteIPC for T
where
    T: ToDataFrame,
{
    fn write_ipc<Writer>(&self, writer: Writer) -> Result<(), Error>
    where
        Writer: Write,
    {
        write_ipc(writer, self)
    }
}

pub trait ReadIPC {
    fn read_ipc<Reader: MmapBytesReader>(reader: Reader) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<T> ReadIPC for T
where
    T: FromDataFrame,
{
    fn read_ipc<Reader: MmapBytesReader>(reader: Reader) -> Result<Self, Error>
    where
        Self: Sized,
    {
        read_ipc(reader)
    }
}
