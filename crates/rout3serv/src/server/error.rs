use std::any::type_name;
use std::fmt::Debug;

use tonic::{Code, Status};

pub trait StatusCodeAndMessage {
    fn status_code_and_message(&self) -> (Code, String);
}

impl StatusCodeAndMessage for h3ron::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        (Code::Internal, format!("{:?}", self))
    }
}

impl StatusCodeAndMessage for s3io::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        match self {
            s3io::Error::H3ron(e) => e.status_code_and_message(),
            s3io::Error::UnidentifiedFileFormat(_)
            | s3io::Error::DataframeInvalidH3IndexType(_, _)
            | s3io::Error::DataframeMissingColumn(_) => (
                Code::FailedPrecondition,
                format!("data inconsistency - {:?}", self),
            ),
            s3io::Error::InvalidS3Region(_)
            | s3io::Error::S3ListObjects(_)
            | s3io::Error::S3GetObject(_)
            | s3io::Error::S3PutObject(_) => {
                (Code::Internal, format!("network error - {:?}", self))
            }
            s3io::Error::S3TLS(_) | s3io::Error::NativeTLS(_) => {
                (Code::Internal, format!("tls error - {:?}", self))
            }
            s3io::Error::UnsupportedH3Resolution(h3_resolution) => (
                Code::OutOfRange,
                format!("unsupported h3 resolution: {}", h3_resolution),
            ),
            _ => (Code::Internal, format!("{:?}", self)),
        }
    }
}

impl StatusCodeAndMessage for h3ron_graph::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        match self {
            h3ron_graph::Error::H3ron(inner) => inner.status_code_and_message(),
            _ => (Code::Internal, format!("{:?}", self)),
        }
    }
}

impl StatusCodeAndMessage for tonic::Status {
    fn status_code_and_message(&self) -> (Code, String) {
        (self.code(), self.message().to_string())
    }
}

macro_rules! error_status_code_impl {
    ($error:ty) => {
        impl StatusCodeAndMessage for $error {
            fn status_code_and_message(&self) -> (Code, String) {
                (Code::Internal, format!("{:?}", self))
            }
        }
    };
}

error_status_code_impl!(gdal::errors::GdalError);
error_status_code_impl!(tokio::task::JoinError);
error_status_code_impl!(eyre::Report);

pub trait ToStatusResult<T> {
    fn to_status_result(self) -> Result<T, Status>;

    fn to_status_result_with_message<MF>(self, code: Code, msg: MF) -> Result<T, Status>
    where
        MF: FnOnce() -> String;
}

impl<T, E> ToStatusResult<T> for Result<T, E>
where
    E: Debug + StatusCodeAndMessage,
{
    fn to_status_result(self) -> Result<T, Status> {
        self.map_err(|e| {
            log::error!("{}: {:?}", type_name::<E>(), format!("{:?}", &e));
            let (code, msg) = e.status_code_and_message();
            Status::new(code, msg)
        })
    }

    fn to_status_result_with_message<MF>(self, code: Code, msg: MF) -> Result<T, Status>
    where
        MF: FnOnce() -> String,
    {
        self.map_err(|e| {
            let msg = msg();
            log::error!("{}: {:?}", msg, e);
            Status::new(code, msg)
        })
    }
}
