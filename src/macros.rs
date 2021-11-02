/// A macro to shorthand the rejection from an endpoint due to a bad request.
macro_rules! reject {
    () => {
        compile_error!("String must be provided to rejection macro!");
    };
    ($arg:tt) => {
        {
            use crate::response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: String::from($arg),
                status: Status::BadRequest,
            }));
        }
    };
    ($($arg:tt)*) => {
        {
            use crate::response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: format!($($arg)*),
                status: Status::BadRequest,
            }));
        }
    };
}

/// A macro to shorthand the rejection from an endpoint due to a server error.
macro_rules! failure {
    () => {
        compile_error!("String must be provided to error macro!");
    };
    ($arg:tt) => {
        {
            use crate::response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: String::from($arg),
                status: Status::InternalServerError,
            }));
        }
    };
    ($($arg:tt)*) => {
        {
            use crate::response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: format!($($arg)*),
                status: Status::InternalServerError,
            }));
        }
    };
}

pub(crate) use {reject, failure};