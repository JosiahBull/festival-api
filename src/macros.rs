//! This module contains ease-of-use macros for the Festival-Api.
//!
//! Each aims to simplify code throughout the project - usually in `main.rs`.

/// A macro to shorthand the rejection from an endpoint due to a bad request.
/// Should be used when you want a quick 400 response to the user.
/// Note that a string must be provided (or used like `format!()`).
///
/// **Examples**
/// ```rust
///     #[get("/")]
///     fn index() -> Response {
///         reject!("You're not cool enough to use this api!");
///     }
/// ```
///
/// /// ```rust
///     #[get("/")]
///     fn index() -> Response {
///         let reason: String = "tall";
///         reject!("You're not {} enough to use this api!", reason);
///     }
/// ```
///
/// If you need more detail in your rejection, you should construct a Response
/// manually for the user. This returns with `ContentType: Plain/Text;`.
macro_rules! reject {
    () => {
        compile_error!("String must be provided to rejection macro!");
    };
    ($arg:tt) => {
        {
            use response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: String::from($arg),
                status: Status::BadRequest,
            }));
        }
    };
    ($($arg:tt)*) => {
        {
            use response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: format!($($arg)*),
                status: Status::BadRequest,
            }));
        }
    };
}

/// A macro to shorthand the rejection from an endpoint due to a server error.
/// /// Should be used when you want a quick 500 response to the user.
/// Note that a string must be provided (or used like `format!()`).
///
/// **Examples**
/// ```rust
///     #[get("/")]
///     fn index() -> Response {
///         failure!("The server had a critical error processing your request!");
///     }
/// ```
///
/// /// ```rust
///     #[get("/")]
///     fn index() -> Response {
///         let reason: String = "it caught fire!";
///         failure!("The server failed to process your request becuase {}", reason);
///     }
/// ```
///
/// If you need a more detailed failure response other htan 500 + a message
/// please construct the response manually.
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
            use response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: format!($($arg)*),
                status: Status::InternalServerError,
            }));
        }
    };
}

pub(crate) use {failure, reject};
