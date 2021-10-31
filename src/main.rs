use rocket::fs::NamedFile;

#[macro_use] extern crate rocket;

mod common;
mod database;

const API_NAME: &str = "Jo";

#[get("/")]
fn index() -> String {
    format!("Welcome to {}'s API for converting text into downloadable wav files! Please make a request to /docs for documentation.", API_NAME)
}

#[get("/docs")]
fn docs() -> NamedFile {
    todo!()
}

#[get("/login")]
fn login() -> String {
    todo!()
}

#[get("/create")]
fn create() -> String {
    todo!()
}

#[get("/convert")]
fn convert() -> String {
    todo!()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, docs])
        .mount("/api/v1/", routes![login, create, convert])
}
