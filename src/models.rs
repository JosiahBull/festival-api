//! Various objects, including database objects, for the api.
use crate::macros::reject;
use crate::schema::*;
use chrono::Utc;
use config::Config;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use response::{Data, Response};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// User credentials, to be used when logging in or creating a new account
#[derive(Deserialize, Serialize, Insertable)]
#[table_name = "users"]
pub struct UserCredentials {
    pub usr: String,
    pub pwd: String,
}

/// Represents a user of this api.
#[derive(Debug, Queryable, QueryableByName, Serialize)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub usr: String,
    pub pwd: String, //hashed
    pub lckdwn: chrono::DateTime<Utc>,
    pub crt: chrono::DateTime<Utc>,
    pub last_accessed: chrono::DateTime<Utc>,
}

/// A request to generate a .wav file from text from a user that has been stored in the db.
/// This is a return object from the reqs table of the database.
#[derive(Queryable, QueryableByName)]
#[table_name = "reqs"]
pub struct GenerationRequest {
    pub id: i32,
    pub usr_id: i32,
    pub crt: chrono::DateTime<Utc>,
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}

/// A request to generate a .wav file from text for a user, to be stored in the db.
/// This is an insertion object for the reqs table of the database.
#[derive(Insertable)]
#[table_name = "reqs"]
pub struct NewGenerationRequest {
    pub usr_id: i32,
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}

/// A phrase package which the user is requesting a speech to be generated for.
#[derive(Deserialize)]
pub struct PhrasePackage {
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}

impl PhrasePackage {
    /// Validates (and attempts to fix) a phrase package.
    /// Returns Ok() if the package is valid, and Err otherwise.
    /// Attempts to correct:
    /// - Speed values larger or smaller than the allowd values
    /// - Speed values that are not divisible by 0.5
    ///
    /// Fails on:
    /// - Invalid language selection
    /// - Invalid file format selection
    /// - Phrase too long
    /// - Phrase contains invalid chars (TBD) //TODO
    /// - Phrase contains invalid phrases
    pub fn validated(&mut self, cfg: &Config) -> Result<(), Response> {
        //Attempt to correct speed values

        if self.speed > cfg.SPEED_MAX_VAL() {
            self.speed = cfg.SPEED_MAX_VAL();
        }
        if self.speed < cfg.SPEED_MIN_VAL() {
            self.speed = cfg.SPEED_MIN_VAL();
        }
        if self.speed % 0.5 != 0.0 {
            self.speed *= 2.0;
            self.speed = self.speed.floor();
            self.speed /= 2.0;
        }

        //Check language selection is valid
        if !cfg.SUPPORTED_LANGS().contains_key(&self.lang) {
            reject!(
                "Provided lang ({}) is not supported by this api!",
                &self.lang
            )
        }

        //Validate fild format selection
        if !cfg.ALLOWED_FORMATS().contains(&self.fmt) {
            reject!(
                "Requested format ({}) is not supported by this api!",
                &self.fmt
            )
        }

        //Check that provided phrase is valid
        if self.word.len() > cfg.WORD_LENGTH_LIMIT() {
            reject!(
                "Phrase is too long! Greater than {} chars",
                cfg.WORD_LENGTH_LIMIT()
            )
        }
        if self.word.is_empty() {
            reject!("No word provided!")
        }

        //Validate that the nothing from the blacklist is present
        let match_phrase = format!(" {} ", self.word);
        for phrase in cfg.BLACKLISTED_PHRASES().iter() {
            if match_phrase.contains(phrase) {
                reject!(
                    "Blacklisted word! Phrase ({}) is not allowed!",
                    phrase.trim()
                );
            }
        }

        for c in self.word.chars() {
            if !cfg.ALLOWED_CHARS().contains(&c) {
                reject!(
                    "Char ({}) is not allowed to be sent to this api! Please try again.",
                    c
                );
            }
        }

        Ok(())
    }
}

/// The claims held by the JWT used for authentication
#[derive(Serialize, Deserialize)]
pub struct Claims {
    /// Expiry
    pub exp: usize,
    /// Issued at
    pub iat: usize,
    /// The id of the user
    pub sub: i32,
}

impl Claims {
    /// Create a new JWT, when provided with the id of the user.
    pub fn new_token(sub: i32, cfg: &Config) -> String {
        let curr_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards!")
            .as_secs() as usize;
        let c: Claims = Claims {
            exp: curr_time + cfg.JWT_EXPIRY_TIME_HOURS() * 60 * 60,
            iat: curr_time,
            sub,
        };
        encode(
            &Header::default(),
            &c,
            &EncodingKey::from_secret(cfg.JWT_SECRET().as_ref()),
        )
        .expect("Failed to generate token")
    }

    ///Attempt to parse claims from a token
    pub fn parse_token(token: &str, cfg: &Config) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(cfg.JWT_SECRET().as_ref()),
            &Validation::default(),
        )
        .map(|o| o.claims)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Claims {
    type Error = Response;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Response> {
        const ACCEPTED_HEADERS: [&str; 3] = ["authorisation", "authorization", "auth"];
        let mut auth_header: Option<String> = None;

        for header in req.headers().iter() {
            if ACCEPTED_HEADERS.contains(&&header.name().as_str().trim().to_lowercase()[..]) {
                auth_header = Some(header.value.to_string());
            }
        }

        if auth_header.is_none() {
            return request::Outcome::Failure((
                Status::Unauthorized,
                Response::TextErr(Data {
                    data: String::from("Authorisation Header Not Present"),
                    status: Status::Unauthorized,
                }),
            ));
        }

        let cfg: &Config = match req.rocket().state::<Config>() {
            Some(f) => f,
            None => {
                return request::Outcome::Failure((
                    Status::InternalServerError,
                    Response::TextErr(Data {
                        data: String::from("Configuration Not Initalised"),
                        status: Status::InternalServerError,
                    }),
                ))
            }
        };

        match Claims::parse_token(&auth_header.unwrap(), cfg) {
            Ok(t) => request::Outcome::Success(t),
            Err(_) => request::Outcome::Failure((
                Status::Unauthorized,
                Response::TextErr(Data {
                    data: String::from("Invalid Auth Token"),
                    status: Status::Unauthorized,
                }),
            )),
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::{Claims, PhrasePackage};
    use crate::common::generate_random_alphanumeric;
    use config::Config;

    #[test]
    fn create_new_token() {
        let _time_tolerance_seconds = 2;

        let cfg = Config::new().unwrap();

        let usr_id = 459;
        let token = Claims::new_token(usr_id, &cfg);
        let claims = Claims::parse_token(&token, &cfg).expect("a valid token");

        assert_eq!(claims.sub, usr_id);
        //TODO validate time claims on the token
    }

    #[test]
    fn validate_success_package() {
        let cfg = Config::new().unwrap();

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
        let cfg = Config::new().unwrap();
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

        // Validate the 0.5 rounding is in place!
        for i in 0..100 {
            let mut pack = PhrasePackage {
                word: String::from("Hello, world!"),
                lang: String::from("en"),
                speed: 0.0 + 0.1 * i as f32,
                fmt: String::from("mp3"),
            };

            pack.validated(&cfg).expect("a valid package");

            assert_eq!(pack.speed % 0.5, 0.0);
        }
    }

    #[test]
    fn validate_failure_package() {
        let cfg = Config::new().unwrap();

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
        let cfg = Config::new().unwrap();

        let mut pack = PhrasePackage {
            word: String::from("hello"),
            lang: String::from("en"),
            speed: cfg.SPEED_MIN_VAL(),
            fmt: String::from("format"),
        };
        match pack.validated(&cfg).unwrap_err() {
            response::Response::TextErr(data) => {
                let inner: String = data.data().to_owned();
                assert_eq!(
                    inner,
                    String::from("Requested format (format) is not supported by this api!")
                )
            }
            _ => panic!("Unexpected response!"),
        }
    }

    #[test]
    fn valid_file_formats() {
        let cfg = Config::new().unwrap();

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
