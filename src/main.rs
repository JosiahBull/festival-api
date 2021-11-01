#[rustfmt::skip] 
mod schema;
mod common;
mod models;
mod response;

#[macro_use] 
extern crate rocket;
#[macro_use] 
extern crate diesel;

use std::env::var;

use rocket::{http::Status, serde::json::Json};
use lazy_static::lazy_static;
use models::{UserCredentials};
use response::{Response, ResponseBuilder};
use diesel::prelude::*;

/// Database connection
#[rocket_sync_db_pools::database("postgres_database")]
pub struct DbConn(diesel::PgConnection);

const API_NAME: &str = "Jo";

lazy_static! {
    static ref JWT_SECRET: String = var("JWT_SECRET").expect("Env var JWT_SECRET not set!");
    static ref JWT_EXPIRY_TIME_HOURS: usize = var("JWT_EXPIRY_TIME_HOURS").expect("Env var JWT_EXPIRY_TIME_HOURS not set!").parse().unwrap();
}

// General Todos
// TODO Implement rate limiting for account creation/login based on ip address. This is especially relevant due to how 
// expensive hashing passwords is compute-wise.
// TODO have another crack at implementing a response api which doesn't require owned values.
// TODO if not found in the global env, static refs should fall back to looking for .env, or Rocket.toml.

#[get("/")]
fn index() -> String {
    format!("Welcome to {}'s API for converting text into downloadable wav files! Please make a request to /docs for documentation.", API_NAME)
}

#[get("/docs")]
fn docs() -> String {
    format!("Api docs not yet setup with automated github actions. Feel free to implement that though if you're up for a challenge!")
}

/// Attempt to login a student
#[post(
    "/login",
    data = "<creds>",
    format = "application/json"
)]
async fn login(conn: DbConn, creds: Json<UserCredentials>) -> Result<Response, Response> {
    let creds = creds.into_inner();

    //Locate the user that is attempting to login
    let user = common::find_user_in_db(&conn, creds.usr).await?;
    if user.is_none() {
        return Err(ResponseBuilder {
            data: "Incorrect Password or Username",
            status: Status::BadRequest,
        }.build())
    }
    let user = user.unwrap();

    //Check that their password hash matches
    let is_valid = common::compare_hashed_strings(creds.pwd, user.pwd)?;
    if !is_valid {
        return Err(ResponseBuilder {
            data: "Incorrect Password or Username",
            status: Status::BadRequest,
        }.build())
    }

    //Update the users last_seen status
    common::update_user_last_seen(&conn, user.id, chrono::offset::Utc::now()).await?;

    Ok(ResponseBuilder {
        data: models::Claims::new_token(user.id),
        status: Status::Ok,
    }.build())
}

/// Attempt to create a new user account
#[post(
    "/create",
    data = "<creds>",
    format = "application/json"
)]
async fn create(conn: DbConn, creds: Json<UserCredentials>) -> Result<Response, Response> {
    let creds = creds.into_inner();

    //Validate password requirements, for now all we check is length
    if creds.pwd.len() < 8 {
        return Err(ResponseBuilder {
            data: "Password Too Short",
            status: Status::BadRequest,
        }
        .build());
    }
    if creds.pwd.len() > 64 {
        return Err(ResponseBuilder {
            data: "Password Too Long",
            status: Status::BadRequest,
        }
        .build());
    }

    //Validate the username isn't taken
    if common::find_user_in_db(&conn, creds.usr.clone()).await?.is_some() {
        return Err(ResponseBuilder {
            data: "Username Taken",
            status: Status::BadRequest,
        }
        .build());
    }

    //Hash Password
    let user = UserCredentials {
        usr: creds.usr,
        pwd: common::hash_string_with_salt(creds.pwd)?,
    };

    //Save account in db
    use schema::users;
    let r: Result<models::User, diesel::result::Error> = conn
        .run(move |c| {
            diesel::insert_into(users::table)
                .values(user)
                .get_result(c)
        })
        .await;

    if let Err(e) = r {
        return Err(ResponseBuilder {
            data: format!("Failed to insert into server {}", e),
            status: Status::InternalServerError,
        }
        .build());
    }

    //Return token to user=
    Ok(ResponseBuilder {
        data: models::Claims::new_token(r.unwrap().id),
        status: Status::Ok,
    }.build())
}

#[get("/convert")]
fn convert() -> String {
    todo!()
}

#[launch]
fn rocket() -> _ {
    //Initalize all globals
    lazy_static::initialize(&JWT_SECRET);
    lazy_static::initialize(&JWT_EXPIRY_TIME_HOURS);

    rocket::build()
        .mount("/", routes![index, docs])
        .mount("/api/v1/", routes![login, create, convert])
        .attach(DbConn::fairing())
}
