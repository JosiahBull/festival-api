use std::{path::PathBuf};
use config::Config;
use rocket::{fairing::AdHoc, request::FromRequest};
use crate::PhrasePackage;
use rocket::async_trait;

/// A trait indicating a tts generator that can be constructed and used to generate audio files from speech
#[async_trait]
pub trait TtsGenerator<'r>: Send + Sync + Sized + 'static {
    type Error: core::fmt::Debug;

    /// Create a new TTS builder
    fn new() -> Result<Self, <Self as TtsGenerator<'r>>::Error>;

    /// Generate an adhoc fairing which can be bound to a launching rocket.
    fn fairing() -> AdHoc where &'r Self: FromRequest<'r> {
        AdHoc::on_ignite("Tts Generator", |rocket| {
            Box::pin(async move {
                let jenny = Self::new().unwrap();
                rocket.manage(jenny)
            })
        })
    }

    /// Generate a phrase utilising the TTS system, with the set parameters
    async fn generate(&self, details: &PhrasePackage, config: &Config) -> Result<PathBuf, <Self as TtsGenerator<'r>>::Error>;
}