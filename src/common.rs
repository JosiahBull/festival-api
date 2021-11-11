//! Common functions used in endpoints. Varied between db interactions and general processing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use diesel::prelude::*;
#[cfg(test)]
use rand::{thread_rng, Rng};
use rocket::http::Status;
use sha2::Digest;

use crate::{config::Config, macros::failure};
use crate::{macros::reject, DbConn};
use response::{Data, Response};

/// Hash a string with a random salt to be stored in the database. Utilizing the argon2id algorithm
/// Followed best practices as laid out here: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
/// Example Usage
/// ```rust
///
/// let unhashed_string = String::from("your-text-here, maybe a password?");
/// let hashed_string = hash_string_with_salt(unhashed_string.clone()).unwrap();
/// let second_hashed_string = hash_string_with_salt(unhashed_string.clone()).unwrap();
///
/// assert_ne!(unhashed_string, hashed_string);
/// assert_ne!(unhashed_string, second_hashed_string);
/// assert_ne!(hashed_string, second_hashed_string);
/// ```
pub fn hash_string_with_salt(s: String) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    //SAFETY: Looking at the source of the argon2 crate, the only way this could fail was if the salt was incorrect
    //which given this function is tested can never happen.
    let hash = argon2.hash_password(s.as_bytes(), &salt).unwrap();
    hash.to_string()
}

/// A function which checks whether the first string can be hashed into the second string.
/// Returns a boolean true if they are the same, and false otherwise.
/// In the event the strings cannot be compared due to an error, returns an Err(response)
/// which may be returned to the user.
/// Example Usage:
/// ```rust
/// let original_string: String = String::from("hello, world");
/// let hashed_string: String = hash_string_with_salt(original_string).unwrap();
///
/// //A valid comparison
/// let result = compare_hashed_strings(original_string, hashed_string);
/// assert!(result);
///
/// //An invalid comparison
/// let result = compared_hashed_strings(String::from("other, string"). hashed_string);
/// assert!(!result);
/// ```
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

/// A search item to be used for finding various database interactions when the flexibility
/// between searching via a users name, and a users id is required.
pub enum SearchItem {
    Name(String),
    Id(i32),
}

/// Attempt to find a user in the database, returns None if the user is unable to be found.
/// Note that the provided name is assumed unique. If multiple results exist, the first will be returned.
/// If the database interaction fails, returns a response which can be shown to the user.
pub async fn find_user_in_db(
    conn: &DbConn,
    search: SearchItem,
) -> Result<Option<crate::models::User>, Response> {
    use crate::schema::users::dsl::*;
    let r: Result<Vec<crate::models::User>, diesel::result::Error> = conn
        .run(move |c| match search {
            SearchItem::Name(s) => users
                .filter(usr.eq(s))
                .limit(1)
                .load::<crate::models::User>(c),
            SearchItem::Id(n) => users
                .filter(id.eq(n))
                .limit(1)
                .load::<crate::models::User>(c),
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
    search: SearchItem,
    time: chrono::DateTime<Utc>,
) -> Result<(), Response> {
    use crate::schema::users::dsl::*;
    let r: Result<crate::models::User, _> = conn
        .run(move |c| match search {
            SearchItem::Name(s) => diesel::update(users.filter(usr.eq(s)))
                .set(last_accessed.eq(time))
                .get_result(c),
            SearchItem::Id(search_id) => diesel::update(users.filter(id.eq(search_id)))
                .set(last_accessed.eq(time))
                .get_result(c),
        })
        .await;

    return match r {
        Ok(_) => Ok(()),
        Err(e) => failure!("Unable to update user due to error {}", e),
    };
}

/// Load a users most recent requests, limited based on the number of requests.
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

/// Returns Ok(()) if the user is not timed out.
/// If the user is timed out returns Err(Response) with a custom message containing
/// the number of seconds the user has left before becoming non-timed out.
pub async fn is_user_timed_out(conn: &DbConn, usr_id: i32, cfg: &Config) -> Result<(), Response> {
    let reqs: Vec<crate::models::GenerationRequest> =
        load_recent_requests(conn, usr_id, cfg.MAX_REQUESTS_ACC_THRESHOLD()).await?;
    if reqs.len() >= cfg.MAX_REQUESTS_ACC_THRESHOLD() {
        //If this user is exempt from rate limits, enforce that now!
        let user = find_user_in_db(conn, SearchItem::Id(usr_id)).await?;
        if let Some(user) = user {
            if let Some(settings) = cfg.USER_SETTINGS().get(&user.usr) {
                if !settings.apply_api_rate_limit {
                    return Ok(());
                }
            }
        } else {
            reject!("User does not exist!");
        }

        //Validate that this user hasn't made too many requests
        let earliest_req_time = get_time_since(reqs.last().unwrap().crt);
        let max_req_time_duration =
            chrono::Duration::minutes(cfg.MAX_REQUESTS_TIME_PERIOD_MINUTES() as i64);

        if earliest_req_time < max_req_time_duration {
            return Err(Response::TextErr(Data {
                data: format!(
                    "Too many requests! You will be able to make another request in {} seconds.",
                    (earliest_req_time - max_req_time_duration)
                        .num_seconds()
                        .abs()
                ),
                status: Status::TooManyRequests,
            }));
        }
    }

    Ok(())
}

/// Uploads the provided phrase_package as a request to the database.
/// This is important for rate limiting, among other things.
pub async fn log_request(
    conn: &DbConn,
    usr_id: i32,
    phrase_package: &crate::models::PhrasePackage,
) -> Result<(), Response> {
    let req = crate::models::NewGenerationRequest {
        usr_id,
        word: phrase_package.word.clone(),
        lang: phrase_package.lang.clone(),
        speed: phrase_package.speed,
        fmt: phrase_package.fmt.clone(),
    };

    use crate::schema::reqs::dsl::reqs;
    let r: Result<usize, diesel::result::Error> = conn
        .run(move |c| diesel::insert_into(reqs).values(req).execute(c))
        .await;

    if let Err(e) = r {
        failure!("Unable to log request to database: {}", e);
    }

    Ok(())
}

/// Get the time (in seconds) since a chrono datetime. Returns a duration which can be negative if the time is in the future.
pub fn get_time_since(time: chrono::DateTime<Utc>) -> chrono::Duration {
    let now = Utc::now();
    now.signed_duration_since(time)
}

/// Generate a randomised alphanumeric (base 62) string of a requested length.
#[cfg(test)]
pub fn generate_random_alphanumeric(length: usize) -> String {
    thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Takes an input reference string, and hashes it using the sha512 algorithm.
/// The resultant value is returned as a string in hexadecmial - meaning it is url and i/o safe.
/// The choice of sha512 over sha256 is that sha512 tends to perform better at  longer strings - which we are likely to
/// encounter with this api. Users the sha2 crate internally for hashing.
pub fn sha_512_hash(input: &str) -> String {
    let mut hasher = sha2::Sha512::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod test {
    use super::{
        compare_hashed_strings, generate_random_alphanumeric, get_time_since, hash_string_with_salt,
    };
    use chrono::Utc;
    use rocket::http::Status;
    use std::collections::HashSet;

    #[test]
    fn test_create_hash_password() {
        //To ensure salt is working
        //Don't set too high, hashing is expensive + time consuming
        let loop_count = 5;

        let pwd = generate_random_alphanumeric(10);

        let mut set: HashSet<String> = HashSet::default();
        for _ in 0..loop_count {
            let hashed_pwd = hash_string_with_salt(pwd.clone());
            if set.contains(&hashed_pwd) {
                panic!("Duplicate key found in set - password not being salted");
            }
            set.insert(hashed_pwd);
        }
    }

    #[test]
    fn test_compare_password() {
        //Ensure that we can compare the hash still!
        let pwd = generate_random_alphanumeric(4);
        let hashed_pwd = hash_string_with_salt(pwd.clone());
        assert!(compare_hashed_strings(pwd, hashed_pwd.clone()).expect("Failed to compare hashes "));
        assert!(!compare_hashed_strings(String::from("hello"), hashed_pwd)
            .expect("Failed to compare hashes "));
    }

    #[test]
    fn failed_password_compare() {
        let pwd = generate_random_alphanumeric(4);

        // This isn't an error that can occur in practice, but it's useful to test that the application is working as expected
        // upon an error being encountered.
        let result = compare_hashed_strings(pwd, String::from("")).expect_err("failed comparison");
        match result {
            response::Response::TextErr(data) => {
                assert_eq!(data.status(), Status::InternalServerError);
                assert_eq!(
                    data.data(),
                    "Failed to compare hashes password hash string too short"
                );
            }
            _ => panic!("Invalid response type!"),
        }
    }

    #[test]
    fn test_get_time_since() {
        let tolerance = 1;

        let time_first = Utc::now();
        let time_future = time_first + chrono::Duration::days(100);
        let time_past = time_first - chrono::Duration::days(100);

        assert!((get_time_since(time_future).num_days() + 100).abs() <= tolerance);
        assert!((get_time_since(time_past).num_days() - 100).abs() <= tolerance);
    }

    #[test]
    fn test_generate_random_alphanumeric() {
        //Note, there is a chance that we *could* get a string which has been generated before.
        //But that chance is infinitesimally small as to be negligible.
        let sample_size = 1000;
        let mut set: HashSet<String> = HashSet::default();
        for _ in 0..sample_size {
            let s = generate_random_alphanumeric(32);
            if set.contains(&s) {
                panic!("Duplicate key found in set");
            }
            set.insert(s);
        }
    }
}
