//! A wrapper for ffmpeg, a library for converting from one audio format to another (among other things).

use std::{collections::HashSet, path::PathBuf, process::Command};

use crate::{ConversionError, ConverterSubprocess};
use async_trait::async_trait;
use config::Config;
use utils::phrase_package::PhrasePackage;

#[derive(Debug)]
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
        desired_speed: f32,
        phrase_package: &PhrasePackage,
        output: &str,
        cfg: &Config,
    ) -> Result<PathBuf, ConversionError> {
        //Slightly convoluted, but this is a hot-path, and creating the path this way minimises allocation.
        let mut converted_file_path = phrase_package.filename_stem_properspeed();
        converted_file_path.reserve(cfg.CACHE_PATH().len() + 5);
        converted_file_path.push_str(".");
        converted_file_path.push_str(output);
        converted_file_path.insert_str(0, "/");
        converted_file_path.insert_str(0, cfg.CACHE_PATH());

        let converted_file_path = PathBuf::from(converted_file_path);

        if converted_file_path.exists() {
            return Ok(converted_file_path);
        }

        let mut input_file_path = phrase_package.filename_stem_basespeed();
        input_file_path.reserve(cfg.CACHE_PATH().len() + 5);
        input_file_path.push_str(".wav");
        input_file_path.insert_str(0, "/");
        input_file_path.insert_str(0, cfg.CACHE_PATH());

        let input_file_path = PathBuf::from(input_file_path);

        if !input_file_path.exists() {
            return Err(ConversionError::NotFound);
        }
        if !input_file_path.is_file() {
            return Err(ConversionError::NotFile);
        }

        let con = Command::new("ffmpeg")
            .arg("-i")
            .arg(input_file_path)
            .arg("-filter:a")
            .arg(format!("atempo={}", desired_speed)) //Change speed of audio
            .arg("-vn") //Strip & disable all video
            .arg(&converted_file_path)
            .output();

        match con {
            Ok(o) if o.status.success() => Ok(converted_file_path),
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
