//! A wrapper for ffmpeg, a library for converting from one audio format to another (among other things).

use std::{collections::HashSet, path::PathBuf, process::Command};

use crate::{ConversionError, ConverterSubprocess};
use async_trait::async_trait;
use config::Config;
use utils::FileHandle;

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
        input: FileHandle,
        desired_speed: f32,
        output: &str,
        cfg: &Config,
    ) -> Result<FileHandle, ConversionError> {
        let pathbuf = input.underlying();

        let converted_file_path = PathBuf::from(format!(
            "{}/{}.{}",
            cfg.CACHE_PATH(),
            pathbuf
                .file_stem()
                .expect("a valid os path")
                .to_str()
                .expect("a valid str"),
            output,
        ));

        if converted_file_path.exists() {
            return Ok(FileHandle::new(
                converted_file_path,
                cfg.MAX_CACHE_SIZE() > 0,
            ));
        }

        if !pathbuf.exists() {
            return Err(ConversionError::NotFound);
        }
        if !pathbuf.is_file() {
            return Err(ConversionError::NotFile);
        }

        let con = Command::new("ffmpeg")
            .arg("-i")
            .arg(pathbuf)
            .arg("-filter:a")
            .arg(format!("atempo={}", desired_speed)) //Change speed of audio
            .arg("-vn") //Strip & disable all video
            .arg(&converted_file_path)
            .output();

        match con {
            Ok(o) if o.status.success() => Ok(FileHandle::new(
                PathBuf::from(converted_file_path),
                cfg.MAX_CACHE_SIZE() > 0,
            )),
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
