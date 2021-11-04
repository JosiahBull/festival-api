#[cfg(not(tarpaulin_include))]
#[rustfmt::skip]
mod schema;
mod cache;
mod common;
mod macros;
mod models;
mod response;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use std::{collections::HashMap, path::Path, process::Command};

use diesel::prelude::*;
use lazy_static::lazy_static;
use macros::{failure, load_env, reject};
use models::UserCredentials;
use response::{Data, Response};
use rocket::{fs::NamedFile, http::Status, serde::json::Json};

#[cfg(not(target_os = "linux"))]
compile_error!("Unable to compile for your platform! This API is only available for Linux due to dependence on Bash commands.");

/// Database connection
#[rocket_sync_db_pools::database("postgres_database")]
pub struct DbConn(diesel::PgConnection);

lazy_static! {
    /// The secret used for fast-hashing JWT's for validation.
    static ref JWT_SECRET: String = load_env!("JWT_SECRET");
    /// The number of hours that a JWT may be used before expiring and forcing the user to revalidate.
    static ref JWT_EXPIRY_TIME_HOURS: usize = load_env!("JWT_EXPIRY_TIME_HOURS");
    /// The name of the api which is sent with certain requests.
    static ref API_NAME: String = load_env!("API_NAME");
    /// The path to the cache for storing .wav files
    static ref CACHE_PATH: String = load_env!("CACHE_PATH");
    /// The maximum length of a phrase that the api will process.
    static ref WORD_LENGTH_LIMIT: usize = load_env!("WORD_LENGTH_LIMIT");
    /// The maximum speed at which a phrase can be read.
    static ref SPEED_MAX_VAL: f32 = load_env!("SPEED_MAX_VAL");
    /// The lowerest speed at which a phrase can be read.
    static ref SPEED_MIN_VAL: f32 = load_env!("SPEED_MIN_VAL");
    /// The maximum requests that an account can make in a given time period established by
    /// `MAX_REQUESTS_TIME_PERIOD_MINUTES`
    static ref MAX_REQUESTS_ACC_THRESHOLD: usize = load_env!("MAX_REQUESTS_ACC_THRESHOLD");
    /// The maximum requests that an ip address can make in a given time period established
    /// by `MAX_REQUESTS_TIME_PERIOD_MINUTES`
    static ref MAX_REQUESTS_IP_THRESHOLD: usize= load_env!("MAX_REQUESTS_IP_THRESHOLD");
    /// The time period for timing out users who make too many requests.
    static ref MAX_REQUESTS_TIME_PERIOD_MINUTES:usize = load_env!("MAX_REQUESTS_TIME_PERIOD_MINUTES");
    /// A list of supported speech languages by this api.
    static ref SUPPORTED_LANGS: HashMap<String, models::Language> = {
        let path = "./config/langs.toml";
        let data = std::fs::read_to_string(path).expect(&format!("Unable to find {}", path));
        let f = data.parse::<toml::Value>().expect(&format!("Unable to parse `{}`", path));

        let languages: &toml::value::Table = f.get("lang")
            .expect(&format!("Unable to parse {}, no langs provided!", path))
            .as_table()
            .expect(&format!("lang tag is not a table in {}", path));

        let mut map: HashMap<String, models::Language> = HashMap::default();
        let keys: Vec<&String> = languages.keys().into_iter().collect();
        for key in keys {
            let lang = languages
                .get(key)
                .expect(&format!("Unable to parse lang {} from {}, is it correctly formatted?", key, path))
                .as_table()
                .expect(&format!("Unable to prase {} as table from {}", key, path));

            let enabled = lang
                .get("enabled")
                .expect(&format!("Unable to parse enabled on {} from {}", key, path))
                .as_bool()
                .expect(&format!("{}'s enabled is not a boolean in {}", key, path));

            let festival_code = lang
                .get("festival_code")
                .expect(&format!("Unable to parse festival_code on {} from {}", key, path))
                .as_str()
                .expect(&format!("{}'s festival_code is not a string in {}", key, path))
                .to_owned();

            let iso_691_code = lang
                .get("iso_691-1_code")
                .expect(&format!("Unable to parse iso-691-1_code on {} from {}", key, path))
                .as_str()
                .expect(&format!("{}'s iso_691-1_code is not a string in {}", key, path))
                .to_owned();

            map.insert(iso_691_code.clone(), models::Language {
                display_name: key.clone(),
                enabled,
                festival_code,
                iso_691_code,
            });
        }

        return map;
    };
    /// The list of supported file-formats, note that wav is the preferred format due to lower cpu usage.
    static ref SUPPORTED_FORMATS: Vec<String> = {
        vec![]
    };
}

// General Todos
// TODO Implement rate limiting for account creation/login based on ip address. This is especially relevant due to how
// expensive hashing passwords is compute-wise.
// TODO the api shouldn't charge for serving files from the cache. If we also provide an endpoint for finding out which
// words are cached, we could allow users to more smartly choose which phrases they wish to display.
// This should also reduce load on the api significant as it'll encourage users to pull common words!

#[get("/")]
fn index() -> String {
    format!("Welcome to {} API for converting text into downloadable wav files! Please make a request to /docs for documentation.", *API_NAME)
}

#[get("/docs")]
fn docs() -> String {
    "Api docs not yet setup with automated github actions. Feel free to implement that though if you're up for a challenge!".to_string()
}

/// Attempt to login a student
#[post("/login", data = "<creds>", format = "application/json")]
async fn login(conn: DbConn, creds: Json<UserCredentials>) -> Result<Response, Response> {
    let creds = creds.into_inner();

    //Locate the user that is attempting to login
    let user: Option<models::User> =
        common::find_user_in_db(&conn, common::SearchItem::Name(creds.usr)).await?;
    if user.is_none() {
        reject!("Incorrect Password or Username")
    }
    let user = user.unwrap();

    //Check that their password hash matches
    let is_valid = common::compare_hashed_strings(creds.pwd, user.pwd)?;
    if !is_valid {
        reject!("Incorrect Password or Username")
    }

    //Update the users last_seen status
    common::update_user_last_seen(&conn, user.id, chrono::offset::Utc::now()).await?;

    Ok(Response::TextOk(Data {
        data: models::Claims::new_token(user.id),
        status: Status::Ok,
    }))
}

/// Attempt to create a new user account
#[post("/create", data = "<creds>", format = "application/json")]
async fn create(conn: DbConn, creds: Json<UserCredentials>) -> Result<Response, Response> {
    let creds = creds.into_inner();

    //Validate password requirements, for now all we check is length
    if creds.pwd.len() < 8 {
        reject!("Password Too Short")
    }
    if creds.pwd.len() > 64 {
        reject!("Password Too Long")
    }

    //Validate the username isn't taken
    if common::find_user_in_db(&conn, common::SearchItem::Name(creds.usr.clone()))
        .await?
        .is_some()
    {
        reject!("Username Taken")
    }

    //Hash Password
    let user = UserCredentials {
        usr: creds.usr,
        pwd: common::hash_string_with_salt(creds.pwd)?,
    };

    //Save account in db
    use schema::users;
    let r: Result<models::User, diesel::result::Error> = conn
        .run(move |c| diesel::insert_into(users::table).values(user).get_result(c))
        .await;

    if let Err(e) = r {
        failure!("Failed to insert into server {}", e)
    }

    //Return token to user
    Ok(Response::TextOk(Data {
        data: models::Claims::new_token(r.unwrap().id),
        status: Status::Created,
    }))
}

/// Expects a phrase package, attempts to convert it to a .mp3 to be returned to the user. Requires authentication to access.
#[post("/convert", data = "<phrase_package>", format = "application/json")]
async fn convert(
    token: Result<models::Claims, Response>,
    conn: DbConn,
    phrase_package: Json<models::PhrasePackage>,
) -> Result<Response, Response> {
    let token = token?;

    // Validate PhrasePackage
    if phrase_package.word.len() > *WORD_LENGTH_LIMIT {
        reject!(
            "Word is too long! Greater than {} chars",
            *WORD_LENGTH_LIMIT
        )
    }
    if phrase_package.word.len() < 1 {
        reject!("No word provided!")
    }
    if !phrase_package.word.bytes().all(|c| !c.is_ascii_digit()) {
        reject!("Cannot have numbers in phrase!")
    }
    if phrase_package.speed > *SPEED_MAX_VAL {
        reject!(
            "Speed values greater than {} are not allowed.",
            *SPEED_MAX_VAL
        )
    }
    if phrase_package.speed < *SPEED_MIN_VAL {
        reject!(
            "Speed values lower than {} are not allowed.",
            *SPEED_MIN_VAL
        )
    }
    if !SUPPORTED_LANGS.contains_key(&phrase_package.lang) {
        reject!(
            "Provided lang ({}) is not supported by this api!",
            &phrase_package.lang
        )
    }

    // Validate that this user hasn't been timed out
    let reqs: Vec<models::GenerationRequest> =
        common::load_recent_requests(&conn, token.sub, *MAX_REQUESTS_ACC_THRESHOLD).await?;
    if reqs.len() == *MAX_REQUESTS_ACC_THRESHOLD {
        //Validate that this user hasn't made too many requests
        let earliest_req_time = common::get_time_since(reqs.last().unwrap().crt);
        let max_req_time_duration =
            chrono::Duration::minutes(*MAX_REQUESTS_TIME_PERIOD_MINUTES as i64);

        if earliest_req_time < max_req_time_duration {
            return Err(Response::TextErr(Data {
                data: format!(
                    "Too many requests! You will be able to make another request in {} seconds.",
                    (earliest_req_time - max_req_time_duration).num_seconds()
                ),
                status: Status::TooManyRequests,
            }));
        }
    }

    // Log this request
    //TODO
    // common::log_request(&conn,)?;

    // Generate the phrase if it isn't in the cache.
    let file_name = format!(
        "{}/{}.wav",
        *CACHE_PATH,
        common::generate_random_alphanumeric(10)
    );
    if !Path::new(&file_name).exists() {
        // Generate a wav file if this file does not already exist.
        let command = format!("echo \"{}\" | text2wave -eval \"({})\" -eval \"(Parameter.set 'Duration_Stretch {})\" -o {}",
            &phrase_package.word,
            &SUPPORTED_LANGS.get(&phrase_package.lang).unwrap().festival_code,
            &phrase_package.speed,
            &file_name
        );

        let word_gen = Command::new("bash")
            .args(["-c", &command])
            .stdout(std::process::Stdio::piped())
            .output();

        if let Err(e) = word_gen {
            error!("Failed to generate wav from provided string. {}", e)
        }
    }

    //Format the file to the desired outputSoi

    let resp_file = match NamedFile::open(&file_name).await {
        Ok(f) => f,
        Err(e) => failure!("Unable to open processed file {}", e),
    };

    //Remove the link on the filesystem, note that as we have an opened NamedFile, that should persist.
    //See https://github.com/SergioBenitez/Rocket/issues/610 for more info.
    //This is temporary pending development of a proper caching system.
    if let Err(e) = rocket::tokio::fs::remove_file(Path::new(&file_name)).await {
        failure!(
            "Unable to temporary file from system prior to response {}",
            e
        )
    };

    //Return the response
    Ok(Response::FileDownload(Data {
        data: resp_file,
        status: Status::Ok,
    }))
}

// struct CacheFairing(crate::cache::Cache<String, models::GenerationRequest, NamedFile>);


#[launch]
fn rocket() -> _ {
    //Initalize all globals
    lazy_static::initialize(&JWT_SECRET);
    lazy_static::initialize(&JWT_EXPIRY_TIME_HOURS);
    lazy_static::initialize(&API_NAME);
    lazy_static::initialize(&CACHE_PATH);
    lazy_static::initialize(&SPEED_MAX_VAL);
    lazy_static::initialize(&SPEED_MIN_VAL);
    lazy_static::initialize(&MAX_REQUESTS_ACC_THRESHOLD);
    lazy_static::initialize(&MAX_REQUESTS_IP_THRESHOLD);
    lazy_static::initialize(&MAX_REQUESTS_TIME_PERIOD_MINUTES);
    lazy_static::initialize(&SUPPORTED_LANGS);
    lazy_static::initialize(&SUPPORTED_FORMATS);

    rocket::build()
        .mount("/", routes![index, docs])
        .mount("/api/v1/", routes![login, create, convert])
        .attach(DbConn::fairing())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::models::UserCredentials;

    use super::common::generate_random_alphanumeric;
    use super::models::Claims;
    use super::rocket;
    use rocket::http::{ContentType, Header, Status};
    use rocket::local::blocking::Client;

    //***** Helper Methods *****//
    fn create_test_account(client: &Client) -> (UserCredentials, String, String) {
        let body = UserCredentials {
            usr: generate_random_alphanumeric(20),
            pwd: String::from("User12356789"),
        };
        let body_json = serde_json::to_string(&body).expect("a json body");

        //Create the account we wish to log into
        let create_response = client
            .post(uri!("/api/v1/create"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();
        assert_eq!(create_response.status(), Status::Created);

        return (body, body_json, create_response.into_string().unwrap());
    }

    //***** Test Methods *****//

    #[test]
    fn test_index() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get(uri!(super::index)).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert!(!response.into_string().unwrap().is_empty());
    }

    #[test]
    fn test_docs() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get(uri!(super::docs)).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert!(!response.into_string().unwrap().is_empty());
    }

    #[test]
    fn login_success() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, body_json, _) = create_test_account(&client);

        //Attempt to login
        let response = client
            .post(uri!("/api/v1/login"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );

        let token = response.into_string().unwrap();
        let _ = Claims::parse_token(&token).expect("a valid token");
    }

    #[test]
    fn login_failures() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (body, body_json, _) = create_test_account(&client);

        let wrong_password_body = body_json.replace(&body.pwd, "incorrect");
        let wrong_username_body = body_json.replace(&body.usr, "incorrect");

        //Login with incorrect username
        let response = client
            .post(uri!("/api/v1/login"))
            .header(ContentType::new("application", "json"))
            .body(&wrong_username_body)
            .dispatch();

        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            response.into_string().unwrap(),
            "Incorrect Password or Username"
        );

        let response = client
            .post(uri!("/api/v1/login"))
            .header(ContentType::new("application", "json"))
            .body(&wrong_password_body)
            .dispatch();

        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            response.into_string().unwrap(),
            "Incorrect Password or Username"
        );
    }

    #[test]
    fn create_success() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let _ = create_test_account(&client);
    }

    #[test]
    fn create_failure_password_short() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let body = UserCredentials {
            usr: generate_random_alphanumeric(20),
            pwd: generate_random_alphanumeric(5),
        };
        let body_json = serde_json::to_string(&body).expect("a json body");

        //Create the account we wish to log into
        let response = client
            .post(uri!("/api/v1/create"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();

        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(response.into_string().unwrap(), "Password Too Short");
    }

    #[test]
    fn create_failure_password_long() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let body = UserCredentials {
            usr: generate_random_alphanumeric(20),
            pwd: generate_random_alphanumeric(100),
        };
        let body_json = serde_json::to_string(&body).expect("a json body");

        //Create the account we wish to log into
        let response = client
            .post(uri!("/api/v1/create"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();

        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(response.into_string().unwrap(), "Password Too Long");
    }

    #[test]
    fn create_failure_taken() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let body = UserCredentials {
            usr: generate_random_alphanumeric(20),
            pwd: generate_random_alphanumeric(30),
        };
        let body_json = serde_json::to_string(&body).expect("a json body");

        //Create an account - shoudl succeed
        let response = client
            .post(uri!("/api/v1/create"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();

        assert_eq!(response.status(), Status::Created);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );

        let token = response.into_string().unwrap();
        let _ = Claims::parse_token(&token).expect("a valid token");

        //Attempt to create account with same username - should fail
        let response = client
            .post(uri!("/api/v1/create"))
            .header(ContentType::new("application", "json"))
            .body(&body_json)
            .dispatch();

        assert_eq!(response.status(), Status::BadRequest);
        assert_eq!(
            response
                .headers()
                .get_one("Content-Type")
                .expect("a content type header"),
            "text/plain; charset=utf-8"
        );
        assert_eq!(response.into_string().unwrap(), "Username Taken");
    }

    #[test]
    fn success_conversion() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = "{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0
        }";

        //Test the generation of the .wav file
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token))
            .body(&body)
            .dispatch();

        let status = response.status();
        if status != Status::Ok {
            panic!("Failed with status {} Body: {}", status, response.into_string().unwrap());
        }

        assert_eq!(
            response.headers().get_one("content-type").unwrap(),
            "audio/mpeg"
        );
        //TODO once filename generation is fixed, actually test for that.
        assert!(response
            .headers()
            .get_one("content-disposition")
            .unwrap()
            .contains("attachment; filename=\""));

        assert_eq!(response.into_bytes().unwrap().len(), 58428);
    }
}
