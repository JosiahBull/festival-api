#![doc = include_str!("../readme.md")]

#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
#[rustfmt::skip]
mod schema;

pub mod common;
pub mod models;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;

use config::Config;
use converter::{Converter, Ffmpeg};
use diesel::prelude::*;
use festvox::{Flite, PhrasePackage, TtsGenerator};
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

/// The base url of the program. This is just a catch-all for those who stumble across the api without knowing what it does.
#[get("/")]
pub fn index(cfg: &Config) -> String {
    format!("Welcome to {}'s TTS API.", cfg.API_NAME())
}

/// Attempts to login a student with provided credentials.
#[post("/login", data = "<creds>", format = "application/json")]
pub async fn login(
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
pub async fn create(
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
pub async fn convert(
    token: Result<models::Claims, Response>,
    conn: DbConn,
    mut phrase_package: Json<PhrasePackage>,
    generator: &Flite,
    converter: &Converter,
    cfg: &Config,
) -> Result<Response, Response> {
    //Validate token
    let token = token?;

    // Validate PhrasePackage
    phrase_package.validated(cfg).map_err(|e| {
        Response::TextErr(Data {
            data: e,
            status: Status::BadRequest,
        })
    })?;
    let phrase_package = phrase_package.into_inner();

    // Validate that this user hasn't been timed out
    common::is_user_timed_out(&conn, token.sub, cfg).await?;

    // Log this request
    common::log_request(&conn, token.sub, &phrase_package).await?;

    // Generate the phrase
    let generated_file = generator
        .generate(&phrase_package, cfg)
        .await
        .map_err(|e| {
            //XXX Displaying internal errors to users...?
            Response::TextErr(Data {
                data: e.to_string(),
                status: Status::InternalServerError,
            })
        })?;

    // Convert the file
    if !converter.is_supported(&phrase_package.fmt) {
        failure!("requested file format is not available on this api, this is a misconfiguration of the deployment")
    }

    match converter.convert(
        generated_file,
        &phrase_package.fmt,
        phrase_package.speed,
        cfg,
    ).await {
        Ok(f) => {
            let resp_file = match NamedFile::open(f.underlying()).await {
                Ok(f) => f,
                Err(e) => failure!("Unable to open processed file {}", e),
            };

            Ok(Response::FileDownload((
                Data {
                    data: resp_file,
                    status: Status::Ok,
                },
                format!("output.{}", phrase_package.fmt),
            )))
        },
        Err(_) => failure!("unable to convert file to desired format due to internal error, try again with request as wav"),
    }
}

#[doc(hidden)]
#[launch]
pub fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/api/", routes![login, create, convert])
        .attach(Config::fairing())
        .attach(DbConn::fairing())
        .attach(Flite::fairing())
        .attach(Converter::fairing(vec![Box::new(
            Ffmpeg::new().expect("a valid ffmpeg instance"),
        )]))
}
