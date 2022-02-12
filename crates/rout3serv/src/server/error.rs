use std::any::type_name;
use std::fmt::Debug;

use tonic::{Code, Status};

pub trait ToStatusResult<T> {
    fn to_status_result(self, code: Code) -> Result<T, Status>;

    fn to_status_message_result<MF>(self, code: Code, msg: MF) -> Result<T, Status>
    where
        MF: FnOnce() -> String;
}

impl<T, E> ToStatusResult<T> for Result<T, E>
where
    E: Debug,
{
    fn to_status_result(self, code: Code) -> Result<T, Status> {
        self.map_err(|e| {
            log::error!("{}: {:?}", type_name::<E>(), format!("{:?}", &e));
            Status::new(code, format!("{:?}", e))
        })
    }

    fn to_status_message_result<MF>(self, code: Code, msg: MF) -> Result<T, Status>
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
