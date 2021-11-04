use crate::response::{Data, Response};
use crate::schema::*;
use crate::{JWT_EXPIRY_TIME_HOURS, JWT_SECRET};
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

#[derive(Queryable, QueryableByName)]
#[table_name = "reqs"]
pub struct GenerationRequest {
    pub id: i32,
    pub usr_id: i32,
    pub crt: chrono::DateTime<Utc>,
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub ip_addr: Vec<u8>,
}

/// A phrase package which the user is requesting a .mp3 for
#[derive(Deserialize)]
pub struct PhrasePackage {
    pub word: String,
    pub lang: String,
    pub speed: f32,
}

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
    use super::Claims;

    #[test]
    fn create_new_token() {
        let _time_tolerance_seconds = 2;

        let usr_id = 459;
        let token = Claims::new_token(usr_id);
        let claims = Claims::parse_token(&token).expect("a valid token");

        assert_eq!(claims.sub, usr_id);
        //TODO validate time claims on the token
    }
}

///// Test Implementation of the cache as a fairing /////
// struct TestCache {
// data: usize,
// db: Option<DbConn>,
// }

// impl TestCache {
// async fn make_request(&self) -> Option<models::User> {
//     if let Some(f) = &self.db {
//         return common::find_user_in_db(f, common::SearchItem::Id(1)).await.unwrap()
//     }
//     None
// }
// }

// #[rocket::async_trait]
// impl rocket::fairing::Fairing for TestCache {
// fn info(&self) -> rocket::fairing::Info {
//     rocket::fairing::Info {
//         name: "Test Cache Implementation",
//         kind: Kind::Ignite
//     }
// }

// async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
//     //Get a db instance
//     let db = DbConn::get_one(&rocket).await.unwrap();

//     //Initialize our test friend
//     let cache = TestCache {
//         data: 5,
//         db: Some(db)
//     };

//     //Save him to a local state
//     let new_rocket = rocket.manage(cache);

//     //Return our succesfully attached fairing!
//     rocket::fairing::Result::Ok(new_rocket)
// }
// }

// impl Default for TestCache {
// fn default() -> Self {
//     TestCache {
//         data: 5,
//         db: None,
//     }
// }
// }

/////fairing cache implemntation tests/////
        // .attach(testcache)
        // .manage(friend)
        // .attach(rocket::fairing::AdHoc::on_liftoff("Freds", |rocket| {
        //     Box::pin(async move {
        //         friend.fetch_update(std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed, |_| Some(4));
        //     })
        // }))