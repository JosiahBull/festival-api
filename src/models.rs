use crate::response::{Response, ResponseBuilder};
use crate::schema::*;
use crate::{JWT_EXPIRY_TIME_HOURS, JWT_SECRET};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// User credentials, to be used when logging in or creating a new account
#[derive(Deserialize, Insertable)]
#[table_name = "users"]
pub struct UserCredentials {
    pub usr: String,
    pub pwd: String,
}

/// Represents a user of this api.
#[derive(Queryable, QueryableByName, Serialize)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub usr: String,
    pub pwd: String, //hashed
    pub lckdwn: chrono::DateTime<Utc>,
    pub crt: chrono::DateTime<Utc>,
    pub last_accessed: chrono::DateTime<Utc>,
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
        .unwrap() // HACK this secret should be loaded in from env
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
                ResponseBuilder {
                    data: "Authorisation Header Not Present",
                    status: Status::Unauthorized,
                }
                .build(),
            ));
        }

        match decode::<Claims>(
            auth_header.unwrap(),
            &DecodingKey::from_secret((*JWT_SECRET).as_ref()),
            &Validation::default(),
        ) {
            Ok(t) => {
                //TODO validate the user hasn't been deleted (check db)
                request::Outcome::Success(t.claims)
            }
            Err(_) => request::Outcome::Failure((
                Status::Unauthorized,
                ResponseBuilder {
                    data: "Invalid Auth Token",
                    status: Status::Unauthorized,
                }
                .build(),
            )),
        }
    }
}
