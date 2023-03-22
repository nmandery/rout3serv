use std::any::type_name;
use std::fmt::Debug;

use tonic::{Code, Status};
use tracing::{error, Level};

pub trait StatusCodeAndMessage {
    fn status_code_and_message(&self) -> (Code, String);

    fn status(&self) -> Status {
        let (code, msg) = self.status_code_and_message();
        Status::new(code, msg)
    }
}

impl StatusCodeAndMessage for crate::io::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        if self.is_not_found() {
            (Code::NotFound, "not found".to_string())
        } else {
            (Code::Internal, format!("IO error: {self:?}"))
        }
    }
}

impl<E> StatusCodeAndMessage for crate::io::memory_cache::FetchError<E>
where
    E: Debug,
{
    fn status_code_and_message(&self) -> (Code, String) {
        (Code::Internal, format!("IO error: {self:?}"))
    }
}

impl StatusCodeAndMessage for hexigraph::error::Error {
    fn status_code_and_message(&self) -> (Code, String) {
        (Code::Internal, format!("{self:?}"))
    }
}

macro_rules! impl_invalid_geom {
    ($type:ty) => {
        impl StatusCodeAndMessage for $type {
            fn status_code_and_message(&self) -> (Code, String) {
                (Code::Internal, format!("invalid geometry: {self:?}"))
            }
        }
    };
}

impl_invalid_geom!(h3o::error::InvalidGeometry);
impl_invalid_geom!(h3o::error::InvalidLatLng);

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

macro_rules! logged_status {
    ($msg:expr, $code: expr, $lvl:expr, $caused_by:expr) => {{
        tracing::event!($lvl, "{}: {:?}", $msg, $caused_by);
        Status::new($code, $msg)
    }};
    ($msg:expr, $code: expr, $lvl:expr) => {{
        tracing::event!($lvl, "{}", $msg);
        Status::new($code, $msg)
    }};
}
pub(crate) use logged_status;

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
        let m = msg();
        self.map_err(|e| logged_status!(m, code, Level::ERROR, &e))
    }
}
