use super::common::*;
use crate::models::{Claims, UserCredentials};
use crate::rocket;
use config::Config;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::uri;
use utils::generate_random_alphanumeric;

#[test]
fn login_success() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, body_json, _) = create_test_account(&client);

    //Attempt to login
    let response = client
        .post(uri!("/api/login"))
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

    let cfg: &Config = client.rocket().state::<Config>().unwrap();

    let token = response.into_string().unwrap();
    let _ = Claims::parse_token(&token, cfg).expect("a valid token");
}

#[test]
fn login_failures() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (body, body_json, _) = create_test_account(&client);

    let wrong_password_body = body_json.replace(&body.pwd, "incorrect");
    let wrong_username_body = body_json.replace(&body.usr, "incorrect");

    //Login with incorrect username
    let response = client
        .post(uri!("/api/login"))
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
        .post(uri!("/api/login"))
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
        .post(uri!("/api/create"))
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
        .post(uri!("/api/create"))
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
        .post(uri!("/api/create"))
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

    let cfg: &Config = client.rocket().state::<Config>().unwrap();

    let token = response.into_string().unwrap();
    let _ = Claims::parse_token(&token, cfg).expect("a valid token");

    //Attempt to create account with same username - should fail
    let response = client
        .post(uri!("/api/create"))
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
