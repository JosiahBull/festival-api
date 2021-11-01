use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use diesel::prelude::*;
use rocket::http::Status;

use crate::response::{Response, ResponseBuilder};
use crate::DbConn;

/// Hash a string with a random salt to be stored in the database.
/// Utilizes the argon2id algorithm
/// Followed best practices as laid out here: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
pub fn hash_string_with_salt(s: String) -> Result<String, Response> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(s.as_bytes(), &salt)
        .map_err(|e| {
            ResponseBuilder {
                data: format!("Failed to create hash {}", e),
                status: Status::InternalServerError,
            }
            .build()
        })?
        .to_string();
    Ok(hash)
}

/// A function which checks whether the first string can be hashed into the second string.
/// Returns a boolean true if they are the same, and false otherwise.
pub fn compare_hashed_strings(orignal: String, hashed: String) -> Result<bool, Response> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&hashed).map_err(|e| {
        ResponseBuilder {
            data: format!("Failed to compare hashes {}", e),
            status: Status::InternalServerError,
        }
        .build()
    })?;
    Ok(argon2
        .verify_password(orignal.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Attempt to find a user in the database, returns None if the user is unable to be found.
/// Note that the provided name is assumed unique. If multiple results exist, the first will be returned.
pub async fn find_user_in_db(
    conn: &DbConn,
    name: String,
) -> Result<Option<crate::models::User>, Response> {
    use crate::schema::users::dsl::*;
    let r = conn
        .run(move |c| {
            users
                .filter(usr.eq(name))
                .limit(1)
                .load::<crate::models::User>(c)
        })
        .await;

    return match r {
        Ok(mut f) if !f.is_empty() => Ok(Some(f.remove(0))),
        Ok(_) => Ok(None),
        Err(e) => Err(ResponseBuilder {
            data: format!("Failed to find user due to error {}", e),
            status: Status::InternalServerError,
        }
        .build()),
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
        Err(e) => Err(ResponseBuilder {
            data: format!("Unable to update user due to error {}", e),
            status: Status::InternalServerError,
        }
        .build()),
    };
}
