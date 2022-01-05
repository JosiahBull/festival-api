//! A wrapper for ffmpeg, a library for converting from one audio format to another (among other things).

use std::{collections::HashSet, ffi::OsStr, path::PathBuf, process::Command};

use crate::{ConversionError, ConverterSubprocess};
use async_trait::async_trait;
use config::Config;
use utils::generate_random_alphanumeric;
//TODO Setup temporary files to be cleared on file close.

pub struct Ffmpeg {}

impl Ffmpeg {
    /// Create a new ffmpeg instance, this involves checking whether ffmpeg is installed.
    /// Due to there only being a single failure case, creating a dedicated error enum
    /// was deemed overkill. A simple string response will be more than sufficent in the
    /// event of a failure.
    pub fn new() -> Result<Self, String> {
        //Check to see if ffmpeg is installed
        let con = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map_err(|_| String::from("ffmpeg not installed"))?;
        if !con.status.success() {
            return Err(String::from("ffmpeg not installed"));
        }

        Ok(Ffmpeg {})
    }
}

#[async_trait]
impl ConverterSubprocess for Ffmpeg {
    fn name(&self) -> &str {
        "ffmpeg"
    }

    fn supported_outputs(&self) -> HashSet<String> {
        HashSet::from([
            String::from("mp3"),
            String::from("wav"),
            String::from("flac"),
            String::from("m4a"),
            String::from("wma"),
            String::from("aac"),
            String::from("aif"),
        ])
    }

    async fn convert(
        &self,
        input: PathBuf,
        output: &str,
        cfg: &Config,
    ) -> Result<PathBuf, ConversionError> {
        if !input.exists() {
            return Err(ConversionError::NotFound);
        }
        if !input.is_file() {
            return Err(ConversionError::NotFile);
        }
        match input.extension() {
            Some(ext) if ext == output => return Ok(input),
            Some(_) => {}
            None => return Err(ConversionError::NoExtension),
        }

        let converted_file_path = format!(
            "{}/temp/{}_{}.{}",
            cfg.CACHE_PATH(),
            input
                .file_name()
                .unwrap_or(OsStr::new(""))
                .to_string_lossy(),
            generate_random_alphanumeric(10),
            output,
        );

        let con = Command::new("ffmpeg")
            .arg("-i")
            .arg(input)
            .arg("-vn") //Strip & disable all video
            .arg(&converted_file_path)
            .output();

        match con {
            Ok(o) if o.status.success() => Ok(PathBuf::from(converted_file_path)),
            Ok(o) => {
                let stdout = String::from_utf8(o.stdout)
                    .unwrap_or_else(|_| "Unable to parse stdout!".into());
                let stderr = String::from_utf8(o.stderr)
                    .unwrap_or_else(|_| "Unable to parse stderr!".into());

                Err(ConversionError::Other(format!(
                    "Failed to convert wav to format due to error.\nStdout: \n{}\nStderr: \n{}",
                    stdout, stderr
                )))
            }
            Err(e) => Err(ConversionError::IoFailure(e)),
        }
    }
}
