#![doc = include_str!("../readme.md")]

#[cfg(not(tarpaulin_include))]
#[rustfmt::skip]
#[doc(hidden)]
mod schema;

mod common;
mod macros;
mod models;
mod response;

//Cache is a WIP, so it's not used currently.
#[allow(dead_code)]
mod cache;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};

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
    /// The path to the cache for storing .wav files.
    static ref CACHE_PATH: String = load_env!("CACHE_PATH");
    /// The path where temporary files are stored, and should be deleted from on a crash.
    static ref TEMP_PATH: String = load_env!("TEMP_PATH");
    /// The maximum length of a phrase that the api will process.
    static ref WORD_LENGTH_LIMIT: usize = load_env!("WORD_LENGTH_LIMIT");
    /// The maximum speed at which a phrase can be read.
    static ref SPEED_MAX_VAL: f32 = load_env!("SPEED_MAX_VAL");
    /// The lowerest speed at which a phrase can be read.
    static ref SPEED_MIN_VAL: f32 = load_env!("SPEED_MIN_VAL");
    /// The maximum requests that an account can make in a given time period established by
    /// `MAX_REQUESTS_TIME_PERIOD_MINUTES`
    static ref MAX_REQUESTS_ACC_THRESHOLD: usize = load_env!("MAX_REQUESTS_ACC_THRESHOLD");
    /// The time period for timing out users who make too many requests.
    static ref MAX_REQUESTS_TIME_PERIOD_MINUTES:usize = load_env!("MAX_REQUESTS_TIME_PERIOD_MINUTES");
    /// A list of supported speech languages by this api.
    static ref SUPPORTED_LANGS: HashMap<String, models::Language> = {
        let path = "./config/langs.toml";
        let data = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("Unable to find {}", path));
        let f = data.parse::<toml::Value>().unwrap_or_else(|_| panic!("Unable to parse `{}`", path));

        let languages: &toml::value::Table = f.get("lang")
            .unwrap_or_else(|| panic!("Unable to parse {}, no langs provided!", path))
            .as_table()
            .unwrap_or_else(|| panic!("lang tag is not a table in {}", path));

        let mut map: HashMap<String, models::Language> = HashMap::default();
        let keys: Vec<&String> = languages.keys().into_iter().collect();
        for key in keys {
            let lang = languages
                .get(key)
                .unwrap_or_else(|| panic!("Unable to parse lang {} from {}, is it correctly formatted?", key, path))
                .as_table()
                .unwrap_or_else(|| panic!("Unable to prase {} as table from {}", key, path));

            let enabled = lang
                .get("enabled")
                .unwrap_or_else(|| panic!("Unable to parse enabled on {} from {}", key, path))
                .as_bool()
                .unwrap_or_else(|| panic!("{}'s enabled is not a boolean in {}", key, path));

            let festival_code = lang
                .get("festival_code")
                .unwrap_or_else(|| panic!("Unable to parse festival_code on {} from {}", key, path))
                .as_str()
                .unwrap_or_else(|| panic!("{}'s festival_code is not a string in {}", key, path))
                .to_owned();

            let iso_691_code = lang
                .get("iso_691-1_code")
                .unwrap_or_else(|| panic!("Unable to parse iso-691-1_code on {} from {}", key, path))
                .as_str()
                .unwrap_or_else(|| panic!("{}'s iso_691-1_code is not a string in {}", key, path))
                .to_owned();

            map.insert(iso_691_code.clone(), models::Language {
                display_name: key.clone(),
                enabled,
                festival_code,
                iso_691_code,
            });
        }

        map
    };
    /// The list of supported file-formats, note that wav is the preferred format due to lower cpu usage.
    static ref ALLOWED_FORMATS: HashSet<String> = {
        let path = "./config/general.toml";
        let data = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", path, e));
        let f = data.parse::<toml::Value>().unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", path, e));

        let table = f.as_table().unwrap_or_else(|| panic!("Unable to parse {} as table.", path));

        let formats = table.get("ALLOWED_FORMATS")
            .unwrap_or_else(|| panic!("Unable to find ALLOWED_FORMATS in {}", path))
            .as_array()
            .unwrap_or_else(|| panic!("ALLOWED_FORMATS in {} is not an array of strings!", path));

        let mut res = HashSet::default();

        for format in formats {
            let string = format
                .as_str()
                .unwrap_or_else(|| panic!("ALLOWED_FORMATS in {} is not an array of strings!", path))
                .to_owned();
            res.insert(string);
        }

        res
    };
}

// General Todos
// TODO Implement timeouts for repeated failed login attempts.
// TODO the api shouldn't charge for serving files from the cache. If we also provide an endpoint for finding out which
// words are cached, we could allow users to more smartly choose which phrases they wish to display.
// This should also reduce load on the api significant as it'll encourage users to pull common words!

/// The base url of the program. This is just a catch-all for those who stumble across the api without knowing what it does.
#[get("/")]
fn index() -> String {
    format!("Welcome to {} API for converting text into downloadable wav files! Please make a request to /docs for documentation.", *API_NAME)
}

/// Returns the OAS docs for this api in an easily downloadable file.
#[get("/docs")]
fn docs() -> String {
    "Api docs not yet setup with automated github actions. Feel free to implement that though if you're up for a challenge!".to_string()
}

/// Attempts to login a student with provided credentials.
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
    common::update_user_last_seen(
        &conn,
        common::SearchItem::Id(user.id),
        chrono::offset::Utc::now(),
    )
    .await?;

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
        pwd: common::hash_string_with_salt(creds.pwd),
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

/// Expects a phrase package, attempts to convert it to a sound file to be returned to the user.
/// Requires an authenticate user account to access. This endpoint also features strict rate limiting
/// as generating .wav files is very resource intensive.
#[post("/convert", data = "<phrase_package>", format = "application/json")]
async fn convert(
    token: Result<models::Claims, Response>,
    conn: DbConn,
    mut phrase_package: Json<models::PhrasePackage>,
) -> Result<Response, Response> {
    //Validate token
    let token = token?;

    // Validate PhrasePackage
    phrase_package.validated()?;

    // Validate that this user hasn't been timed out
    common::is_user_timed_out(&conn, token.sub).await?;

    // Log this request
    common::log_request(&conn, token.sub, &phrase_package).await?;

    // Generate the phrase
    let file_name_base = common::generate_random_alphanumeric(10);

    let file_name_wav = format!("{}/{}.wav", *CACHE_PATH, &file_name_base,);

    if !Path::new(&file_name_wav).exists() {
        // Generate a wav file if this file does not already exist.
        // TODO make this secure!
        let command = format!("echo \"{}\" | text2wave -eval \"({})\" -eval \"(Parameter.set 'Duration_Stretch {})\" -o '{}'",
            &phrase_package.word,
            &SUPPORTED_LANGS.get(&phrase_package.lang).unwrap().festival_code,
            &phrase_package.speed,
            &file_name_wav
        );
        //TODO refactor this erorr handling into another function
        let word_gen = Command::new("bash").args(["-c", &command]).output();

        if let Err(e) = word_gen {
            failure!("Failed to generate wav from provided string. {}", e)
        }
        let word_gen = word_gen.unwrap();

        if !word_gen.status.success() {
            let stdout = String::from_utf8(word_gen.stdout)
                .unwrap_or_else(|_| "Unable to parse stdout!".into());
            let stderr = String::from_utf8(word_gen.stderr)
                .unwrap_or_else(|_| "Unable to parse stderr!".into());

            failure!("Failed to generate wav from provided string due to error.\nStdout: \n{}\nStderr: \n{}", stdout, stderr)
        }
    }

    let mut converted_file = file_name_wav.clone();

    //Format the file to the desired output
    if phrase_package.fmt != "wav" {
        //Carry out conversion
        converted_file = format!(
            "{}/temp/{}.{}",
            *CACHE_PATH, &file_name_base, phrase_package.fmt
        );

        let con = Command::new("sox")
            .arg(&file_name_wav)
            .arg(&converted_file)
            .output();

        //TODO refactor this erorr handling into another function that can be tested individually
        if let Err(e) = con {
            failure!("Failed to generate wav from provided string. {}", e)
        }
        let con = con.unwrap();

        if !con.status.success() {
            let stdout =
                String::from_utf8(con.stdout).unwrap_or_else(|_| "Unable to parse stdout!".into());
            let stderr =
                String::from_utf8(con.stderr).unwrap_or_else(|_| "Unable to parse stderr!".into());

            failure!("Failed to generate wav from provided string due to error.\nStdout: \n{}\nStderr: \n{}", stdout, stderr)
        }
    }

    let resp_file = match NamedFile::open(&converted_file).await {
        Ok(f) => f,
        Err(e) => failure!("Unable to open processed file {}", e),
    };

    //Remove the link on the filesystem, note that as we have an opened NamedFile, that should persist.
    //See https://github.com/SergioBenitez/Rocket/issues/610 for more info.
    //This is temporary pending development of a proper caching system.
    if let Err(e) = rocket::tokio::fs::remove_file(Path::new(&file_name_wav)).await {
        failure!(
            "Unable to remove temporary file from system prior to response {}",
            e
        )
    };

    if file_name_wav != converted_file {
        if let Err(e) = rocket::tokio::fs::remove_file(Path::new(&converted_file)).await {
            failure!(
                "Unable to remove temporary file from system prior to response {}",
                e
            )
        };
    }

    //Return the response
    Ok(Response::FileDownload((
        Data {
            data: resp_file,
            status: Status::Ok,
        },
        format!("output.{}", phrase_package.fmt),
    )))
}

// struct CacheFairing(crate::cache::Cache<String, models::GenerationRequest, NamedFile>);

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    //Initalize all globals
    lazy_static::initialize(&JWT_SECRET);
    lazy_static::initialize(&JWT_EXPIRY_TIME_HOURS);
    lazy_static::initialize(&API_NAME);
    lazy_static::initialize(&CACHE_PATH);
    lazy_static::initialize(&TEMP_PATH);
    lazy_static::initialize(&SPEED_MAX_VAL);
    lazy_static::initialize(&SPEED_MIN_VAL);
    lazy_static::initialize(&MAX_REQUESTS_ACC_THRESHOLD);
    lazy_static::initialize(&MAX_REQUESTS_TIME_PERIOD_MINUTES);
    lazy_static::initialize(&SUPPORTED_LANGS);
    lazy_static::initialize(&ALLOWED_FORMATS);

    rocket::build()
        .mount("/", routes![index, docs])
        .mount("/api/v1/", routes![login, create, convert])
        .attach(DbConn::fairing())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod rocket_tests {
    use super::common::generate_random_alphanumeric;
    use super::models::{Claims, UserCredentials};
    use super::rocket;
    use super::ALLOWED_FORMATS;
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

        (body, body_json, create_response.into_string().unwrap())
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

    // use rocket::tokio;
    // #[rocket::tokio::test(flavor = "multi_thread", worker_threads = 16)]
    // async fn gen_speed() {
    //     let mut handles = vec![];
    //     for i in 0..200 {
    //         let t = tokio::spawn(async move {
    //             let command = format!("echo \"{}\" | text2wave -o {}",
    //                 "The university of auckland is cool!",
    //                 &format!("./cache/file-{}.wav", i)
    //             );

    //             let word_gen = std::process::Command::new("bash")
    //                 .args(["-c", &command])
    //                 .output();

    //             word_gen.unwrap();
    //         });
    //         handles.push(t);
    //     }

    //     futures::future::join_all(handles).await;
    // }

    #[test]
    fn success_conversion() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = "{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"wav\"
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
            panic!(
                "Failed with status {} \nBody: \n{}\n",
                status,
                response.into_string().unwrap()
            );
        }

        assert_eq!(
            response.headers().get_one("content-type").unwrap(),
            "audio/mpeg"
        );

        assert_eq!(
            response.headers().get_one("content-disposition").unwrap(),
            "attachment; filename=\"output.wav\""
        );
        assert_eq!(response.into_bytes().unwrap().len(), 63840);
    }

    #[test]
    fn test_limits() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = "{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"wav\"
        }";

        for _ in 0..2 {
            //Test the generation of the .wav file
            let response = client
                .post(uri!("/api/v1/convert"))
                .header(ContentType::new("application", "json"))
                .header(Header::new("Authorisation", token.clone()))
                .body(&body)
                .dispatch();
            let status = response.status();
            if status != Status::Ok {
                panic!(
                    "Failed with status {} \nBody: \n{}\n",
                    status,
                    response.into_string().unwrap()
                );
            }
        }

        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token))
            .body(&body)
            .dispatch();

        let status = response.status();
        if status != Status::TooManyRequests {
            panic!(
                "Failed with status {} \nBody: \n{}\n",
                status,
                response.into_string().unwrap()
            );
        }

        //TODO this could check for a tolerance on the seconds number?
        assert!(response
            .into_string()
            .unwrap()
            .contains("Too many requests! You will be able to make another request in"));
    }

    /// Validate that all file format options work as intended
    #[test]
    fn test_every_format() {
        for format in ALLOWED_FORMATS.iter() {
            let client = Client::tracked(rocket()).expect("valid rocket instance");
            let (_, _, token) = create_test_account(&client);

            let body = format!(
                "{{
                \"word\": \"The University of Auckland\",
                \"lang\": \"en\",
                \"speed\": 1.0,
                \"fmt\": \"{}\"
            }}",
                format
            );

            //Generate a 'generic' file and validate the response is correct
            let response = client
                .post(uri!("/api/v1/convert"))
                .header(ContentType::new("application", "json"))
                .header(Header::new("Authorisation", token))
                .body(&body)
                .dispatch();

            let status = response.status();
            if status != Status::Ok {
                panic!(
                    "Failed with status {} \nBody: \n{}\n",
                    status,
                    response.into_string().unwrap()
                );
            }
            let expected = format!("attachment; filename=\"output.{}\"", format);
            assert_eq!(
                response.headers().get_one("content-disposition").unwrap(),
                expected
            );
        }
    }

    /// Validate that all languages are accepted by the api
    #[test]
    fn test_every_lang() {
        //TODO
    }

    /// Ensure that incorrect tokens fail as expected
    #[test]
    fn test_invalid_auth_tokens() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = "{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"wav\"
        }";

        //Test No Header
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .body(&body)
            .dispatch();

        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(
            response.into_string().unwrap(),
            "Authorisation Header Not Present"
        );

        //Test Invalid Token

        let bad_token = format!("a{}", &token);

        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", bad_token))
            .body(&body)
            .dispatch();

        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(response.into_string().unwrap(), "Invalid Auth Token");

        //Test Invalid Header
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorization", token))
            .body(&body)
            .dispatch();

        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(
            response.into_string().unwrap(),
            "Authorisation Header Not Present"
        );
    }

    /// A simple test which ensures that an invalid file format fails out as expected.
    #[test]
    fn test_invalid_formats() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = "{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"this-will-never-exist\"
        }";

        //Generate a 'generic' file and validate the response is correct
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token))
            .body(&body)
            .dispatch();

        let status = response.status();
        if status != Status::BadRequest {
            panic!(
                "Failed with status {} \nBody: \n{}\n",
                status,
                response.into_string().unwrap()
            );
        }

        let body = response.into_string().expect("a valid body");
        assert_eq!(
            body,
            String::from("Requested format (this-will-never-exist) is not supported by this api!")
        );
    }
}
