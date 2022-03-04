use config::Config;
use serde::Deserialize;

/// A phrase package which the user is requesting a speech to be generated for.
#[derive(Deserialize)]
pub struct PhrasePackage {
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}

impl PhrasePackage {
    /// Generate a filename, minus the file extension
    pub fn filename_stem_properspeed(&self) -> String {
        crate::sha_256_hash(&format!("{}_{}_{}", self.word, self.lang, self.speed))
    }

    /// Collect the name of the file pre-conversion or speed change
    pub fn filename_stem_basespeed(&self) -> String {
        crate::sha_256_hash(&format!("{}_{}_1.0", self.word, self.lang))
    }

    /// Validates (and attempts to fix) a phrase package.
    /// Returns Ok() if the package is valid, and Err otherwise.
    /// Attempts to correct:
    /// - Speed values larger or smaller than the allowed values
    /// - Speed values that are not divisible by 0.5
    ///
    /// Fails on:
    /// - Invalid language selection
    /// - Invalid file format selection
    /// - Phrase too long
    /// - Phrase contains invalid chars (TBD)
    /// - Phrase contains invalid phrases
    pub fn validated(&mut self, cfg: &Config) -> Result<(), String> {
        //Attempt to correct speed values

        if self.speed % 0.5 != 0.0 {
            self.speed *= 2.0;
            self.speed = self.speed.floor();
            self.speed /= 2.0;
        }
        if self.speed > cfg.SPEED_MAX_VAL() {
            self.speed = cfg.SPEED_MAX_VAL();
        }
        if self.speed < cfg.SPEED_MIN_VAL() {
            self.speed = cfg.SPEED_MIN_VAL();
        }

        //Check language selection is valid
        if !cfg.SUPPORTED_LANGS().contains_key(&self.lang) {
            return Err(format!(
                "Provided lang ({}) is not supported by this api!",
                &self.lang
            ));
        }

        //Validate fild format selection
        if !cfg.ALLOWED_FORMATS().contains(&self.fmt) {
            return Err(format!(
                "Requested format ({}) is not supported by this api!",
                &self.fmt
            ));
        }

        //Check that provided phrase is valid
        if self.word.len() > cfg.WORD_LENGTH_LIMIT() {
            return Err(format!(
                "Phrase is too long! Greater than {} chars",
                cfg.WORD_LENGTH_LIMIT()
            ));
        }
        if self.word.is_empty() {
            return Err(String::from("No word provided!"));
        }

        //Validate that the nothing from the blacklist is present
        let match_phrase = format!(" {} ", self.word);
        for phrase in cfg.BLACKLISTED_PHRASES().iter() {
            if match_phrase.contains(phrase) {
                return Err(format!(
                    "Blacklisted word! Phrase ({}) is not allowed!",
                    phrase.trim()
                ));
            }
        }

        for c in self.word.chars() {
            if !cfg.ALLOWED_CHARS().contains(&c) {
                return Err(format!(
                    "Char ({}) is not allowed to be sent to this api! Please try again.",
                    c
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::PhrasePackage;
    use crate::generate_random_alphanumeric;
    use config::Config;

    #[test]
    fn validate_success_package() {
        let cfg = Config::new(PathBuf::from("../../../config")).unwrap();

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: cfg.SPEED_MAX_VAL(),
            fmt: String::from("mp3"),
        };
        pack.validated(&cfg).expect("a valid package");

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };
        pack.validated(&cfg).expect("a valid package");

        let mut pack = PhrasePackage {
            word: String::from("H"),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };
        pack.validated(&cfg).expect("a valid package");

        let mut pack = PhrasePackage {
            word: generate_random_alphanumeric(cfg.WORD_LENGTH_LIMIT()),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };
        pack.validated(&cfg).expect("a valid package");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn validate_correction_package() {
        let cfg = Config::new(PathBuf::from("../../../config")).unwrap();
        // Validate the min value correct is in place!

        //We can't run this test if the min value is 0.0!
        if cfg.SPEED_MIN_VAL() <= 0.0 {
            panic!("WARNING: TEST UNABLE TO BE RUN AS SPEED_MIN_VAL < 0.0!");
        }

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL() - 0.1,
            fmt: String::from("mp3"),
        };

        // Validate the max value correct is in place!
        pack.validated(&cfg).expect("a valid package");
        assert_eq!(pack.speed, cfg.SPEED_MIN_VAL());

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: cfg.SPEED_MAX_VAL() + 0.1,
            fmt: String::from("mp3"),
        };

        pack.validated(&cfg).expect("a valid package");
        assert_eq!(pack.speed, cfg.SPEED_MAX_VAL());

        // Validate the 0.1 rounding is in place!
        for i in 0..500 {
            let mut pack = PhrasePackage {
                word: String::from("Hello, world!"),
                lang: String::from("en"),
                speed: 0.0 + 0.35 * i as f32,
                fmt: String::from("mp3"),
            };

            pack.validated(&cfg).expect("a valid package");

            assert_eq!(pack.speed % 0.5, 0.0);
        }
    }

    #[test]
    fn validate_failure_package() {
        let cfg = Config::new(PathBuf::from("../../../config")).unwrap();

        // Validate that empty string fails
        let mut pack = PhrasePackage {
            word: String::from(""),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };

        pack.validated(&cfg).expect_err("should be too short");

        //Test string too long
        let mut pack = PhrasePackage {
            word: generate_random_alphanumeric(cfg.WORD_LENGTH_LIMIT() + 1),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };

        pack.validated(&cfg).expect_err("should be too long");

        //Test unsupported lang
        let mut pack = PhrasePackage {
            word: String::from("a wiord"),
            lang: String::from("adfadlfjalk"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("mp3"),
        };

        pack.validated(&cfg).expect_err("should be invalid lang");
    }

    #[test]
    fn invalid_file_formats() {
        let cfg = Config::new(PathBuf::from("../../../config")).unwrap();

        let mut pack = PhrasePackage {
            word: String::from("hello"),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("format"),
        };
        if let Err(e) = pack.validated(&cfg) {
            assert_eq!(
                e,
                String::from("Requested format (format) is not supported by this api!")
            )
        } else {
            panic!("Unexpected response!")
        }
    }

    #[test]
    fn valid_file_formats() {
        let cfg = Config::new(PathBuf::from("../../../config")).unwrap();

        for format in cfg.ALLOWED_FORMATS().iter() {
            let mut pack = PhrasePackage {
                word: String::from("hello"),
                lang: String::from("en"),
                speed: cfg.SPEED_MIN_VAL(),
                fmt: format.clone(),
            };

            pack.validated(&cfg).expect("a valid pack");
        }
    }
}
