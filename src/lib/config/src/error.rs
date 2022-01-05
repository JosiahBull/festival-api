use std::{
    convert::Infallible,
    num::{ParseFloatError, ParseIntError},
};

#[derive(Debug)]
pub enum ConfigError {
    ParseIntError(ParseIntError),
    ParseFloatError(ParseFloatError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ConfigError::ParseIntError(_) => write!(f, "failed to parse int"),
            ConfigError::ParseFloatError(_) => write!(f, "failed to parse float"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::ParseIntError(ref e) => Some(e),
            Self::ParseFloatError(ref e) => Some(e),
        }
    }
}

impl From<Infallible> for ConfigError {
    fn from(_: Infallible) -> Self {
        unreachable!("Infallible errors can never be produced")
    }
}

impl From<ParseIntError> for ConfigError {
    fn from(e: ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}

impl From<ParseFloatError> for ConfigError {
    fn from(e: ParseFloatError) -> Self {
        Self::ParseFloatError(e)
    }
}
