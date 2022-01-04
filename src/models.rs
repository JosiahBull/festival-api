//! Various objects, including database objects, for the api.
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
    use super::{Claims};
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
}
