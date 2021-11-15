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

/// Defines a JSON value to be received through the api
/// This value will be automatically defined, to access the value inside of your function
/// use the sister function `unwrap_json` to access the internal value.
//TODO make this a proc macro so users are not required to manually unwrap things.
/// Example
/// ```rust
///     #[get("/", data = "<item>")]
///     fn index(item: json!(String)) -> Result<Response, Response> {
///         unwrap_json!(item)
///     }
/// ```
macro_rules! json {
    ($arg:tt) => {
        Result<Json<$arg>, rocket::serde::json::Error<'_>>
    };
}


macro_rules! unwrap_json {
    ($arg:tt) => {
        {
            if let Err(e) = $arg {
                match e {
                    rocket::serde::json::Error::Io(e) => failure!("Failed to parse request body due to an i/o error. This is usually a problem with the server, and not with your request. Try again later. \n {}", e),
                    rocket::serde::json::Error::Parse(bdy, err) => {
                        if bdy.is_empty() {
                            reject!("No json body found!");    
                        }
                        return Err(Response::TextErr(Data {
                            data: format!("Invalid json body {}", err),
                            status: Status::UnprocessableEntity
                        }));
                    },
                }
            }
            $arg.unwrap().into_inner()
        }
    };
}

pub(crate) use {failure, reject, json, unwrap_json};