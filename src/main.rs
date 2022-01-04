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
mod models;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use std::{
    path::Path,
    process::{Command, Stdio},
};

use config::Config;
use diesel::prelude::*;
use festvox::{PhrasePackage, Festival, TtsGenerator};
use macros::{failure, reject};
use models::UserCredentials;
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
    creds: Json<UserCredentials>,
    cfg: &Config,
) -> Result<Response, Response> {
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
        data: models::Claims::new_token(user.id, cfg),
        status: Status::Ok,
    }))
}

/// Attempt to create a new user account
#[post("/create", data = "<creds>", format = "application/json")]
async fn create(
    conn: DbConn,
    creds: Json<UserCredentials>,
    cfg: &Config,
) -> Result<Response, Response> {
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
        data: models::Claims::new_token(r.unwrap().id, cfg),
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
    mut phrase_package: Json<PhrasePackage>,
    generator: &Festival,
    cfg: &Config,
) -> Result<Response, Response> {
    //Validate token
    let token = token?;

    // Validate PhrasePackage
    phrase_package
        .validated(cfg)
        .map_err(|e| {
            Response::TextErr(Data {
                data: e,
                status: Status::BadRequest
            })
        })?;

    // Validate that this user hasn't been timed out
    common::is_user_timed_out(&conn, token.sub, cfg).await?;

    // Log this request
    common::log_request(&conn, token.sub, &phrase_package).await?;

    // Generate the phrase

    let generated_file = generator
        .generate(
            &phrase_package,
            cfg,
        ).await
        .map_err(|e| {
            //XXX Displaying internal errors to users...?
            Response::TextErr(Data {
                data: e.to_string(),
                status: Status::InternalServerError
            })
        })?;

    let resp_file = match NamedFile::open(&generated_file).await {
        Ok(f) => f,
        Err(e) => failure!("Unable to open processed file {}", e),
    };

    //Remove the link on the filesystem, note that as we have an opened NamedFile, that should persist.
    //See https://github.com/SergioBenitez/Rocket/issues/610 for more info.
    // if file_name_wav != converted_file {
    //     if let Err(e) = rocket::tokio::fs::remove_file(Path::new(&converted_file)).await {
    //         failure!(
    //             "Unable to remove temporary file from system prior to response {}",
    //             e
    //         )
    //     };
    // }

    // Return the response
    Ok(Response::FileDownload((
        Data {
            data: resp_file,
            status: Status::Ok,
        },
        format!("output.{}", phrase_package.fmt),
    )))
}

#[doc(hidden)]
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, docs])
        .mount("/api/", routes![login, create, convert])
        .attach(Config::fairing())
        .attach(DbConn::fairing())
        .attach(Festival::fairing())
}
