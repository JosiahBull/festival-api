use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use diesel::prelude::*;
use rocket::http::Status;
use rand::{thread_rng, Rng};

use crate::macros::failure;
use crate::response::{Data, Response};
use crate::DbConn;

/// Hash a string with a random salt to be stored in the database.
/// Utilizes the argon2id algorithm
/// Followed best practices as laid out here: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
pub fn hash_string_with_salt(s: String) -> Result<String, Response> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(s.as_bytes(), &salt).map_err(|e| {
        Response::TextErr(Data {
            data: format!("Failed to create hash {}", e),
            status: Status::InternalServerError,
        })
    })?;
    Ok(hash.to_string())
}

/// A function which checks whether the first string can be hashed into the second string.
/// Returns a boolean true if they are the same, and false otherwise.
pub fn compare_hashed_strings(orignal: String, hashed: String) -> Result<bool, Response> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&hashed).map_err(|e| {
        Response::TextErr(Data {
            data: format!("Failed to compare hashes {}", e),
            status: Status::InternalServerError,
        })
    })?;
    Ok(argon2
        .verify_password(orignal.as_bytes(), &parsed_hash)
        .is_ok())
}

pub enum SearchItem {
    Name(String),
    #[allow(dead_code)]
    Id(i32),
}

/// Attempt to find a user in the database, returns None if the user is unable to be found.
/// Note that the provided name is assumed unique. If multiple results exist, the first will be returned.
pub async fn find_user_in_db(
    conn: &DbConn,
    name: SearchItem,
) -> Result<Option<crate::models::User>, Response> {
    use crate::schema::users::dsl::*;
    let r: Result<Vec<crate::models::User>, diesel::result::Error> = conn
        .run(move |c| {
            return match name {
                SearchItem::Name(s) => users
                    .filter(usr.eq(s))
                    .limit(1)
                    .load::<crate::models::User>(c),
                SearchItem::Id(n) => users
                    .filter(id.eq(n))
                    .limit(1)
                    .load::<crate::models::User>(c),
            };
        })
        .await;

    return match r {
        Ok(mut f) if !f.is_empty() => Ok(Some(f.remove(0))),
        Ok(_) => Ok(None),
        Err(e) => failure!("Failed to find user due to error {}", e),
    };
}

/// Attempts to find and then update a user with a new timestamp.
pub async fn update_user_last_seen(
    conn: &DbConn,
    search_id: i32,
    time: chrono::DateTime<Utc>,
) -> Result<(), Response> {
    use crate::schema::users::dsl::*;
    let r: Result<crate::models::User, _> = conn
        .run(move |c| {
            diesel::update(users.filter(id.eq(search_id)))
                .set(last_accessed.eq(time))
                .get_result(c)
        })
        .await;

    return match r {
        Ok(_) => Ok(()),
        Err(e) => failure!("Unable to update user due to error {}", e),
    };
}

/// Load a users most recent requests, limited based on 
pub async fn load_recent_requests(
    conn: &DbConn,
    search_id: i32,
    limit: usize,
) -> Result<Vec<crate::models::GenerationRequest>, Response> {
    if limit == 0 {
        return Ok(vec![]);
    }

    use crate::schema::reqs::dsl::*;
    let r = conn
        .run(move |c| {
            reqs.filter(usr_id.eq(search_id))
                .order(crt.desc())
                .limit(limit as i64)
                .load::<crate::models::GenerationRequest>(c)
        })
        .await;

    return match r {
        Ok(f) => Ok(f),
        Err(e) => failure!("Unable to collect recent requests due to error {}", e),
    };
}

/// Get the time (in seconds) since a chrono datetime. Returns a duration which can be negative if the time is in the future.
pub fn get_time_since(time: chrono::DateTime<Utc>) -> chrono::Duration {
    let now = Utc::now();
    now.signed_duration_since(time)
}

/// Generate a randomised alphanumeric (base 62) string of a requested length.
pub fn generate_random_alphanumeric(length: usize) -> String {
    thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}