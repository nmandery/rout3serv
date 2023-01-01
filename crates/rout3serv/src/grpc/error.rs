use std::any::type_name;
use std::fmt::Debug;

use tonic::{Code, Status};
use tracing::error;
use tracing::log::{log, Level};

pub trait StatusCodeAndMessage {
    fn status_code_and_message(&self) -> (Code, String);

    fn status(&self) -> Status {
        let (code, msg) = self.status_code_and_message();
        Status::new(code, msg)
    }
}

impl StatusCodeAndMessage for h3ron::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        (Code::Internal, format!("{:?}", self))
    }
}

impl StatusCodeAndMessage for crate::io::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        if self.is_not_found() {
            (Code::NotFound, "not found".to_string())
        } else {
            (Code::Internal, format!("IO error: {:?}", self))
        }
    }
}

impl<E> StatusCodeAndMessage for crate::io::memory_cache::FetchError<E>
where
    E: Debug,
{
    fn status_code_and_message(&self) -> (Code, String) {
        (Code::Internal, format!("IO error: {:?}", self))
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

impl StatusCodeAndMessage for h3ron_polars::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        match self {
            h3ron_polars::Error::H3ron(inner) => inner.status_code_and_message(),
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

error_status_code_impl!(tokio::task::JoinError);
//error_status_code_impl!(anyhow::Error);
error_status_code_impl!(polars_core::error::PolarsError);

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
            error!("{}: {:?}", type_name::<E>(), format!("{:?}", &e));
            e.status()
        })
    }

    fn to_status_result_with_message<MF>(self, code: Code, msg: MF) -> Result<T, Status>
    where
        MF: FnOnce() -> String,
    {
        self.map_err(|e| logged_status_with_cause(msg(), code, Level::Error, &e))
    }
}

#[inline(always)]
pub fn logged_status<A>(msg: A, code: Code, level: tracing::log::Level) -> Status
where
    A: AsRef<str>,
{
    log!(level, "{}", msg.as_ref());
    Status::new(code, msg.as_ref())
}

#[inline(always)]
pub fn logged_status_with_cause<A, E>(
    msg: A,
    code: Code,
    level: tracing::log::Level,
    caused_by: &E,
) -> Status
where
    A: AsRef<str>,
    E: Debug,
{
    log!(level, "{}: {:?}", msg.as_ref(), caused_by);
    Status::new(code, msg.as_ref())
}
