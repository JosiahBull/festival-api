//! This module contains ease-of-use macros for the Festival-Api.
//!
//! Each aims to simplify code throughout the project - usually in `main.rs`.

/// A macro to shorthand the rejection from an endpoint due to a bad request.
/// Should be used when you want a quick 400 response to the user.
/// Note that a string must be provided (or used like `format!()`).
///
/// **Examples**
/// ```rust
///     fn main() {
///         #[get("/")]
///         fn index() -> Response {
///             reject!("You're not cool enough to use this api!");
///         }
///     }
/// ```
///
/// /// ```rust
///     fn main() {
///         #[get("/")]
///         fn index() -> Response {
///             let reason: String = "tall";
///             reject!("You're not {} enough to use this api!", reason);
///         }
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
/// /// Should be used when you want a quick 500 response to the user.
/// Note that a string must be provided (or used like `format!()`).
///
/// **Examples**
/// ```rust
///     fn main() {
///         #[get("/")]
///         fn index() -> Response {
///             failure!("The server had a critical error processing your request!");
///         }
///     }
/// ```
///
/// /// ```rust
///     fn main() {
///         #[get("/")]
///         fn index() -> Response {
///             let reason: String = "it caught fire!";
///             failure!("The server failed to process your request becuase {}", reason);
///         }
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
            use crate::response::{Data, Response};
            return Err(Response::TextErr(Data {
                data: format!($($arg)*),
                status: Status::InternalServerError,
            }));
        }
    };
}

/// A macro to load configuration from the environment.
///
/// Attempts to load from multiple sources falling back in this order:
/// 1. Load from environment
/// 2. Load from `./config/general.toml`
/// 3. panic!
///
/// This macro recommend for use in conjunction with lazy static, as these variables like to be loaded/parsed
/// at runtime, not at compile-time.
///
/// **Example**
/// ```rust
///     lazy_static! {
///         static ref NUMBER_SHOES: usize = load_env!("NUMBER_SHOES");
///     }
///
///     fn main() {
///         lazy_static::initialize(&NUMBER_SHOES);
///         println!("The number of shoes is {}", *NUMBER_SHOES);
///     }
/// ```
/// A variety of types are supported for implicit conversion, look [here](https://docs.rs/toml/0.5.8/toml/value/enum.Value.html#impl-From%3C%26%27a%20str%3E) for a dedicated list of these types.
///
/// Internally this macro relies on `toml::value::Value.try_into()` for type conversion.
///
macro_rules! load_env {
    () => {
        compile_error!("String must be provided to load_env macro!");
    };
    ($arg:tt) => {
        {
            use std::env::var;
            let env_name: &str = $arg;

            //1. Attempt to load from env
            //Attempt to load with truecase
            if let Ok(d) = var(env_name) {
                return d.parse().expect("a parsed value")
            }
            //Attempt to load with uppercase
            if let Ok(d) = var(env_name.to_uppercase()) {
                return d.parse().expect("a parsed value")
            }
            //Attempt to load with lowercase
            if let Ok(d) = var(env_name.to_lowercase()) {
                return d.parse().expect("a parsed value")
            }

            //2. Attempt to load from /config/general.toml
            fn load_from_toml(name: &str) -> Result<toml::Value, String> {
                let file_path = "./config/general.toml";
                let data = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
                let f = data.parse::<toml::Value>().map_err(|e| e.to_string())?;

                return if let Some(k) = f.get(name) {
                    Ok(k.to_owned())
                } else {
                    Err(String::from("Key Not found in ./config/general.toml"))
                }
            }
            //Attempt to load with truecase
            if let Ok(d) = load_from_toml(&env_name) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }
            //Attempt to load with uppercase
            if let Ok(d) = load_from_toml(&env_name.to_uppercase()) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }
            //Attempt to load lowercase
            if let Ok(d) = load_from_toml(&env_name.to_lowercase()) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }

            //3. Failure
            panic!("Env {} not found in environment, ./.env or /config/general.toml. Program start failed.", env_name)
        }
    };
    ($($arg:tt)*) => {
        compile_error!("Too many arguments provided to load_env macro!");
    };
}

pub(crate) use {failure, load_env, reject};
