use std::{convert::Infallible, path::PathBuf, process::Command};

use rocket::request::FromRequest;
use utils::phrase_package::PhrasePackage;

use crate::TtsGenerator;

#[derive(Debug)]
pub enum FliteError {
    UnableToStart(std::io::Error),
    IoFailure(std::io::Error),
    ProcessError(String),
}

impl std::fmt::Display for FliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error occured within generator")
    }
}

impl std::error::Error for FliteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            FliteError::UnableToStart(ref e) => Some(e),
            FliteError::IoFailure(ref e) => Some(e),
            _ => None,
        }
    }
}

pub struct Flite {}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Flite {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let state = req
            .rocket()
            .state::<Flite>()
            .expect("flite fairing attached");
        rocket::request::Outcome::Success(state)
    }
}

#[rocket::async_trait]
impl<'r> TtsGenerator<'r> for Flite {
    type Error = FliteError;

    fn new() -> Result<Self, Self::Error> {
        Ok(Self {})
    }

    async fn generate(
        &self,
        details: &PhrasePackage,
        config: &config::Config,
    ) -> Result<PathBuf, Self::Error> {
        let file_path = PathBuf::from(config.CACHE_PATH()).join(format!("{}.wav", details.filename_stem_basespeed()));

        if file_path.exists() && file_path.is_file() {
            return Ok(file_path);
        }

        let word_gen = Command::new("flite")
            .arg("-voice")
            .arg(
                &config
                    .SUPPORTED_LANGS()
                    .get(&details.lang)
                    .unwrap()
                    .festival_code,
            )
            .arg("-t")
            .arg(format!("\"{}\"", &details.word))
            .arg("-o")
            .arg(&file_path)
            .spawn();

        let word_gen = match word_gen {
            Ok(f) => f.wait_with_output(),
            Err(e) => return Err(FliteError::UnableToStart(e)),
        };

        match word_gen {
            Ok(f) if f.status.success() => {}
            Ok(f) => {
                let stdout = String::from_utf8(f.stdout)
                    .unwrap_or_else(|_| "Unable to parse stdout!".into());
                let stderr = String::from_utf8(f.stderr)
                    .unwrap_or_else(|_| "Unable to parse stderr!".into());

                return Err(
                    FliteError::ProcessError(
                        format!("Failed to generate wav from provided string due to error.\nStdout: \n{}\nStderr: \n{}", stdout, stderr)
                    )
                );
            }
            Err(e) => return Err(FliteError::IoFailure(e)),
        }

        Ok(file_path)
    }
}
