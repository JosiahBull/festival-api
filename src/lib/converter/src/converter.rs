use async_trait::async_trait;
use config::Config;
use rocket::{error, fairing::AdHoc, request::FromRequest};
use std::{collections::HashSet, convert::Infallible};
use utils::FileHandle;

#[derive(Debug)]
pub enum ConversionError {
    NotFile,
    NotFound,
    NoExtension,
    Other(String),
    IoFailure(std::io::Error),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NotFile => write!(f, "path is not file"),
            Self::NotFound => write!(f, "file not found"),
            Self::NoExtension => write!(f, "file does not have extension"),
            Self::Other(ref s) => write!(f, "{}", s),
            Self::IoFailure(_) => write!(f, "error occured when reading from stdout"),
        }
    }
}

impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::IoFailure(ref e) => Some(e),
            _ => None,
        }
    }
}

#[async_trait]
pub trait ConverterSubprocess: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn supported_outputs(&self) -> HashSet<String>;
    async fn convert(
        &self,
        input: FileHandle,
        target_speed: f32,
        output: &str,
        cfg: &Config,
    ) -> Result<FileHandle, ConversionError>;
}

pub struct Converter {
    subs: Vec<Box<dyn ConverterSubprocess>>,
    supported_types: HashSet<String>,
}

impl Converter {
    pub fn fairing(subs: Vec<Box<dyn ConverterSubprocess>>) -> AdHoc {
        AdHoc::on_ignite("Tts Generator", |rocket| {
            Box::pin(async move {
                let mut supported_types = HashSet::default();
                for sub in &subs {
                    supported_types.extend(sub.supported_outputs())
                }

                rocket.manage(Converter {
                    subs,
                    supported_types,
                })
            })
        })
    }

    pub fn is_supported(&self, to_check: &String) -> bool {
        self.supported_types.contains(to_check)
    }

    //XXX improve error responses
    pub async fn convert(
        &self,
        input: FileHandle,
        desired_format: &str, //XXX accept any?
        target_speed: f32,
        cfg: &Config,
    ) -> Result<FileHandle, ()> {
        for sub in self.subs.iter() {
            if sub.supported_outputs().contains(desired_format) {
                match sub
                    .convert(input.clone(), target_speed, desired_format, cfg)
                    .await
                {
                    Ok(res) => return Ok(res),
                    Err(e) => error!("Error in converter `{}` occured {:?}", e, sub.name()),
                }
            }
        }
        Err(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Converter {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let state = req
            .rocket()
            .state::<Converter>()
            .expect("converter fairing attached");
        rocket::request::Outcome::Success(state)
    }
}

// mod tests {
// #[test]
// fn basic_functionality() {}
// }
