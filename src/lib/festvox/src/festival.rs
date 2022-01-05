use crate::{PhrasePackage, TtsGenerator};
use config::Config;
use rocket::request::FromRequest;
use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug)]
pub enum FestivalError {
    ConversionError(String),
}

impl std::fmt::Display for FestivalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error occured within generator")
    }
}

//XXX
impl std::error::Error for FestivalError {}

#[derive(Debug)]
pub struct Festival {}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Festival {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let state = req
            .rocket()
            .state::<Festival>()
            .expect("festival fairing attached");
        rocket::request::Outcome::Success(state)
    }
}

#[rocket::async_trait]
impl<'r> TtsGenerator<'r> for Festival {
    type Error = FestivalError;

    fn new() -> Result<Self, Self::Error> {
        Ok(Festival {})
    }

    async fn generate(
        &self,
        details: &PhrasePackage,
        config: &Config,
    ) -> Result<PathBuf, <Self as TtsGenerator<'r>>::Error> {
        // Create the basefile name to be stored on the system. The solution to this is to hash the provided
        // name into something that is always unique, but can be easily stored on the underlying system.
        let file_name_base: String = utils::sha_512_hash(&format!(
            "{}_{}_{}",
            &details.word, &details.lang, &details.speed
        ));

        let file_name_wav = format!("{}/{}.wav", config.CACHE_PATH(), &file_name_base);

        if !Path::new(&file_name_wav).exists() {
            // Generate a wav file if this file does not already exist.
            let input = format!("\"{}\"", &details.word);

            let echo_child = Command::new("echo")
                .arg(input)
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start echo process");

            let echo_out = echo_child.stdout.expect("failed to start echo process");

            let word_gen = Command::new("text2wave")
                .arg("-eval")
                .arg(format!(
                    "({})",
                    config
                        .SUPPORTED_LANGS()
                        .get(&details.lang)
                        .unwrap()
                        .festival_code
                ))
                .arg("-eval")
                .arg(format!(
                    "(Parameter.set 'Duration_Stretch {})",
                    &details.speed
                ))
                .arg("-o")
                .arg(&file_name_wav)
                .stdin(Stdio::from(echo_out))
                .spawn()
                .expect("failed text2wave command");

            let word_gen = word_gen.wait_with_output();

            if let Err(e) = word_gen {
                return Err(FestivalError::ConversionError(format!(
                    "Failed to generate wav from provided string. {}",
                    e
                )));
            }
            let word_gen = word_gen.unwrap();

            if !word_gen.status.success() {
                let stdout = String::from_utf8(word_gen.stdout)
                    .unwrap_or_else(|_| "Unable to parse stdout!".into());
                let stderr = String::from_utf8(word_gen.stderr)
                    .unwrap_or_else(|_| "Unable to parse stderr!".into());

                return Err(
                    FestivalError::ConversionError(
                        format!("Failed to generate wav from provided string due to error.\nStdout: \n{}\nStderr: \n{}", stdout, stderr)
                    )
                );
            }
        }

        let output = PathBuf::from(file_name_wav);
        Ok(output)
    }
}
