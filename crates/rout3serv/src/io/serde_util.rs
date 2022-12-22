//! Convenience serialization helpers
//!
//! The serialization aims to be fast and allows to apply a LZ4 compression.
//!
use std::io;
use std::panic::catch_unwind;

use serde::Serialize;
use zstd::{Decoder, Encoder};

use crate::io::Error;

///
/// When `compress` is set to `true` ZSTD compression is applied.
pub fn serialize_into<W, T: ?Sized>(writer: W, value: &T, compress: bool) -> Result<(), Error>
where
    W: io::Write,
    T: Serialize,
{
    if compress {
        let mut encoder = Encoder::new(writer, 3)?;
        bincode::serialize_into(&mut encoder, value)?;
        encoder.finish()?;
    } else {
        bincode::serialize_into(writer, value)?;
    };
    Ok(())
}

/// deserialize. When the reader contains ZSTD-compressed data, it
/// is decompressed on-the-fly.
///
/// Has the benefit over `deserialize_from` of not requiring a wrapping `std::io::Cursor` to
/// get support for `Seek`.
pub fn deserialize_from_byte_slice<T>(byte_slice: &[u8]) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    catch_unwind(|| {
        let mut decoder = Decoder::new(byte_slice)?;
        match bincode::deserialize_from(&mut decoder) {
            Err(_) => bincode::deserialize_from(byte_slice).map_err(Error::from),
            Ok(des) => Ok(des),
        }
    })
    .map_err(|_| Error::DeserializePanic)?
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{deserialize_from_byte_slice, serialize_into};

    fn roundtrip(compress: bool) {
        let data = vec![1_i32, 2, 3, 4];
        let mut data_bytes: Vec<u8> = vec![];
        serialize_into(Cursor::new(&mut data_bytes), &data, compress).unwrap();
        assert!(!data_bytes.is_empty());
        let data2: Vec<i32> = deserialize_from_byte_slice(&data_bytes).unwrap();
        assert_eq!(data, data2);
    }

    #[test]
    fn test_roundtrip_no_compression() {
        roundtrip(false);
    }

    #[test]
    fn test_roundtrip_compression() {
        roundtrip(true);
    }
}
