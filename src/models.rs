//! Various objects, including database objects, for the api.
use crate::macros::reject;
use crate::response::{Data, Response};
use crate::schema::*;
use crate::{
    ALLOWED_FORMATS, JWT_EXPIRY_TIME_HOURS, JWT_SECRET, SPEED_MAX_VAL, SPEED_MIN_VAL,
    SUPPORTED_LANGS, WORD_LENGTH_LIMIT,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
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
    pub fn validated(&mut self) -> Result<(), Response> {
        //Attempt to correct speed values
        if self.speed > *SPEED_MAX_VAL {
            self.speed = *SPEED_MAX_VAL;
        }
        if self.speed < *SPEED_MIN_VAL {
            self.speed = *SPEED_MIN_VAL;
        }
        if self.speed % 0.5 != 0.0 {
            self.speed *= 2.0;
            self.speed = self.speed.floor();
            self.speed /= 2.0;
        }

        //Check language selection is valid
        if !SUPPORTED_LANGS.contains_key(&self.lang) {
            reject!(
                "Provided lang ({}) is not supported by this api!",
                &self.lang
            )
        }

        //Validate fild format selection
        if !ALLOWED_FORMATS.contains(&self.fmt) {
            reject!(
                "Requested format ({}) is not supported by this api!",
                &self.fmt
            )
        }

        //Check that provided phrase is valid
        if self.word.len() > *WORD_LENGTH_LIMIT {
            reject!(
                "Phrase is too long! Greater than {} chars",
                *WORD_LENGTH_LIMIT
            )
        }
        if self.word.is_empty() {
            reject!("No word provided!")
        }
        if !self.word.bytes().all(|c| !c.is_ascii_digit()) {
            reject!("Cannot have numbers in phrase!")
        }

        Ok(())
    }
}

/// Represents a possible language that the api may convert text into.
/// This is loaded on boot from `./config/langs.toml`.
pub struct Language {
    pub display_name: String,
    pub iso_691_code: String,
    pub festival_code: String,
    pub enabled: bool,
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
    pub fn new_token(sub: i32) -> String {
        let curr_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards!")
            .as_secs() as usize;
        let c: Claims = Claims {
            exp: curr_time + *JWT_EXPIRY_TIME_HOURS * 60 * 60,
            iat: curr_time,
            sub,
        };
        encode(
            &Header::default(),
            &c,
            &EncodingKey::from_secret((*JWT_SECRET).as_ref()),
        )
        .unwrap() //TODO fix unwraps
    }

    ///Attempt to parse claims from a token
    pub fn parse_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret((*JWT_SECRET).as_ref()),
            &Validation::default(),
        )
        .map(|o| o.claims)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Claims {
    type Error = Response;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Response> {
        //TODO improve the headers here, so we will check for Authorization along with Authorisation.
        //Should help the americanally challenged of us...
        let auth_header = req.headers().get_one("Authorisation");
        if auth_header.is_none() {
            return request::Outcome::Failure((
                Status::Unauthorized,
                Response::TextErr(Data {
                    data: String::from("Authorisation Header Not Present"),
                    status: Status::Unauthorized,
                }),
            ));
        }

        match Claims::parse_token(auth_header.unwrap()) {
            Ok(t) => {
                //TODO validate the user hasn't been deleted (check db)
                request::Outcome::Success(t)
            }
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
    use super::{ALLOWED_FORMATS, SPEED_MAX_VAL, SPEED_MIN_VAL, WORD_LENGTH_LIMIT};
    use crate::common::generate_random_alphanumeric;

    #[test]
    fn create_new_token() {
        let _time_tolerance_seconds = 2;

        let usr_id = 459;
        let token = Claims::new_token(usr_id);
        let claims = Claims::parse_token(&token).expect("a valid token");

        assert_eq!(claims.sub, usr_id);
        //TODO validate time claims on the token
    }

    #[test]
    fn validate_success_package() {
        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: *SPEED_MAX_VAL,
            fmt: String::from("mp3"),
        };
        pack.validated().expect("a valid package");

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };
        pack.validated().expect("a valid package");

        let mut pack = PhrasePackage {
            word: String::from("H"),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };
        pack.validated().expect("a valid package");

        let mut pack = PhrasePackage {
            word: generate_random_alphanumeric(*WORD_LENGTH_LIMIT)
                .chars()
                .map(|x| {
                    if !x.is_numeric() {
                        return x;
                    }
                    'a'
                })
                .collect(),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };
        pack.validated().expect("a valid package");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn validate_correction_package() {
        // Validate the min value correct is in place!

        //We can't run this test if the min value is 0.0!
        if *SPEED_MIN_VAL <= 0.0 {
            panic!("WARNING: TEST UNABLE TO BE RUN AS SPEED_MIN_VAL < 0.0!");
        }

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL - 0.1,
            fmt: String::from("mp3"),
        };

        // Validate the max value correct is in place!
        pack.validated().expect("a valid package");
        assert_eq!(pack.speed, *SPEED_MIN_VAL);

        let mut pack = PhrasePackage {
            word: String::from("Hello, world!"),
            lang: String::from("en"),
            speed: *SPEED_MAX_VAL + 0.1,
            fmt: String::from("mp3"),
        };

        pack.validated().expect("a valid package");
        assert_eq!(pack.speed, *SPEED_MAX_VAL);

        // Validate the 0.5 rounding is in place!
        for i in 0..100 {
            let mut pack = PhrasePackage {
                word: String::from("Hello, world!"),
                lang: String::from("en"),
                speed: 0.0 + 0.1 * i as f32,
                fmt: String::from("mp3"),
            };

            pack.validated().expect("a valid package");

            assert_eq!(pack.speed % 0.5, 0.0);
        }
    }

    #[test]
    fn validate_failure_package() {
        // Validate that empty string fails
        let mut pack = PhrasePackage {
            word: String::from(""),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };

        pack.validated().expect_err("should be too short");

        //Test string too long
        let mut pack = PhrasePackage {
            word: generate_random_alphanumeric(*WORD_LENGTH_LIMIT + 1)
                .chars()
                .map(|x| {
                    if !x.is_numeric() {
                        return x;
                    }
                    'a'
                })
                .collect(),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };

        pack.validated().expect_err("should be too long");

        //Test unsupported lang
        let mut pack = PhrasePackage {
            word: String::from(""),
            lang: String::from("adfadlfjalk"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };

        pack.validated().expect_err("should be invalid lang");

        //Check that numbers in phrase fails
        let mut pack = PhrasePackage {
            word: String::from("adfae12312"),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("mp3"),
        };

        pack.validated().expect_err("should be too short");
    }

    #[test]
    fn invalid_file_formats() {
        let mut pack = PhrasePackage {
            word: String::from("hello"),
            lang: String::from("en"),
            speed: *SPEED_MIN_VAL,
            fmt: String::from("format"),
        };
        match pack.validated().unwrap_err() {
            crate::response::Response::TextErr(data) => {
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
        for format in ALLOWED_FORMATS.iter() {
            let mut pack = PhrasePackage {
                word: String::from("hello"),
                lang: String::from("en"),
                speed: *SPEED_MIN_VAL,
                fmt: format.clone(),
            };

            pack.validated().expect("a valid pack");
        }
    }
}
