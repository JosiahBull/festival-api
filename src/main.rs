#![doc = include_str!("../readme.md")]

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
mod tests;

#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
#[rustfmt::skip]
mod schema;

mod common;
mod macros;
mod models;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use std::{path::Path, process::{Command, Stdio}};

use config::Config;
use diesel::prelude::*;
use macros::{failure, reject, json, unwrap_json};
use models::{UserCredentials, PhrasePackage};
use response::{Data, Response};
use rocket::{fs::NamedFile, http::Status, serde::json::Json};

#[cfg(not(target_os = "linux"))]
compile_error!("Unable to compile for your platform! This API is only available for Linux due to dependence on Bash commands.");

/// Database connection
#[rocket_sync_db_pools::database("postgres_database")]
pub struct DbConn(diesel::PgConnection);

// General Todos
// TODO Implement timeouts for repeated failed login attempts.
// TODO the api shouldn't charge for serving files from the cache. If we also provide an endpoint for finding out which
// words are cached, we could allow users to more smartly choose which phrases they wish to display.
// This should also reduce load on the api significant as it'll encourage users to pull common words!
// TODO create proc-macro for generating fallback endpoints

/// The base url of the program. This is just a catch-all for those who stumble across the api without knowing what it does.
#[get("/")]
fn index(cfg: &Config) -> String {
    format!("Welcome to {} API for converting text into downloadable wav files! Please make a request to /docs for documentation.", cfg.API_NAME())
}

/// Returns the OAS docs for this api in an easily downloadable file.
#[get("/docs")]
fn docs() -> String {
    "Api docs not yet setup with automated github actions. Feel free to implement that though if you're up for a challenge!".to_string()
}

/// Attempts to login a student with provided credentials.
#[post("/login", data = "<creds>", format = "application/json")]
async fn login(
    conn: DbConn,
    creds: json!(UserCredentials),
    cfg: &Config,
) -> Result<Response, Response> {
    //Validate that the provided user data is correct.
    let creds = unwrap_json!(creds);

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
        data: models::Claims::new_token(user.id, cfg),
        status: Status::Ok,
    }))
}

/// A fallback endpoint, this is called if the user provides and incorrect content type on their request 
/// e.g. not application/json.
//TODO Generate with procedural macro
#[post("/login", rank = 2)]
fn fallback_login() -> Response {
    Response::TextErr(Data {
        data: String::from("Remember to specify your content type as application/json!"),
        status: Status::UnsupportedMediaType
    })
}

/// Attempt to create a new user account
#[post("/create", data = "<creds>", format = "application/json")]
async fn create(
    conn: DbConn,
    creds: json!(UserCredentials),
    cfg: &Config,
) -> Result<Response, Response> {
    //Validate that the provided user data is correct.
    let creds = unwrap_json!(creds);

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
        data: models::Claims::new_token(r.unwrap().id, cfg),
        status: Status::Created,
    }))
}

/// A fallback endpoint, this is called if the user provides and incorrect content type on their request 
/// e.g. not application/json.
//TODO Generate with procedural macro
#[post("/create", rank = 2)]
fn fallback_create() -> Response {
    Response::TextErr(Data {
        data: String::from("Remember to specify your content type as application/json!"),
        status: Status::UnsupportedMediaType
    })
}

/// Expects a phrase package, attempts to convert it to a sound file to be returned to the user.
/// Requires an authenticate user account to access. This endpoint also features strict rate limiting
/// as generating .wav files is very resource intensive.
#[post("/convert", data = "<phrase_package>", format = "application/json")]
async fn convert(
    token: Result<models::Claims, Response>,
    conn: DbConn,
    phrase_package: json!(PhrasePackage),
    cfg: &Config,
) -> Result<Response, Response> {
    // Validate token
    let token = token?;

    // Validate that the phrase_package is valid json
    let mut phrase_package = unwrap_json!(phrase_package);

    // Validate PhrasePackage
    phrase_package.validated(cfg)?;

    // Validate that this user hasn't been timed out
    common::is_user_timed_out(&conn, token.sub, cfg).await?;

    // Log this request
    common::log_request(&conn, token.sub, &phrase_package).await?;

    // Generate the phrase

    // Create the basefile name to be stored on the system. The solution to this is to hash the provided
    // name into something that is always unique, but can be easily stored on the underlying system.
    let temp = format!(
        "{}_{}_{}",
        &phrase_package.word, &phrase_package.lang, &phrase_package.speed
    );
    let file_name_base: String = common::sha_512_hash(&temp);

    let file_name_wav = format!("{}/{}.wav", cfg.CACHE_PATH(), &file_name_base,);

    if !Path::new(&file_name_wav).exists() {
        // Generate a wav file if this file does not already exist.

        let input = format!("\"{}\"", &phrase_package.word);

        let echo_child = Command::new("echo")
            .arg(input)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start echo process");

        let echo_out = echo_child.stdout.expect("big sad");

        let word_gen = Command::new("text2wave")
            .arg("-eval")
            .arg(format!(
                "({})",
                cfg.SUPPORTED_LANGS()
                    .get(&phrase_package.lang)
                    .unwrap()
                    .festival_code
            ))
            .arg("-eval")
            .arg(format!(
                "(Parameter.set 'Duration_Stretch {})",
                &phrase_package.speed
            ))
            .arg("-o")
            .arg(&file_name_wav)
            .stdin(Stdio::from(echo_out))
            .spawn()
            .expect("failed text2wave command");

        let word_gen = word_gen.wait_with_output();

        //TODO refactor this error handling into another function
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
            cfg.CACHE_PATH(),
            &file_name_base,
            phrase_package.fmt
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


#[post("/convert", rank = 2)]
fn fallback_convert() -> Response {
    Response::TextErr(Data {
        data: String::from("Remember to specify your content type as application/json!"),
        status: Status::UnsupportedMediaType
    })
}

#[catch(404)]
fn not_found_catcher(_status: Status, req: &rocket::Request) -> String {
    //TODO write a catcher which can search endpoints and return a suggestion.
    // If a user does "GET /login", we should return a 404 but with a message such as. "GET /login does not exist, but POST /login does! Try that?"
    // This sort of feedback could be very useful for a learning user to interact with the api.
    format!("Unable to find {}, double check that you are using the correct request method with a valid url!", req.uri())
}

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, docs])
        .register("/", catchers![not_found_catcher])
        .mount("/api/", routes![login, create, convert]) //Main endpoints
        .mount("/api/", routes![fallback_login, fallback_create, fallback_convert]) //Fallback endpoints
        .attach(Config::fairing())
        .attach(DbConn::fairing())
}
