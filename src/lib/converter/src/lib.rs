//! This library handles conversions from one file format to another.
//! It is designed to convert specifically from .wav to other file formats, other file formats will be rejected.
//! It will attempt to convert the file until it either fails or succesfully converts the file.

mod converter;
mod ffmpeg;

pub use crate::converter::*;
pub use crate::ffmpeg::*;
pub use async_trait;
