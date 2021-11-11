use std::collections::HashSet;

use crate::models::UserCredentials;
use crate::rocket;
use rand::{Rng, thread_rng};
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::uri;

//***** Helper Methods *****//
pub fn create_test_account(client: &Client) -> (UserCredentials, String, String) {
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

/// A simple struct which allows a property on toml to be changed.
pub struct AlteredToml(String);

impl AlteredToml {
    // TODO make this smarter by allowing a key-value replace, rather than a specific string.
    // This should maek the test more robust.
    pub fn new(search: &str, replace: &str) -> Self {
        let path = "./config/general.toml";
        let data = std::fs::read_to_string(path).unwrap();

        //Search through data
        let new_str = data.replace(search, replace);

        std::fs::write("./config/general-test.toml", new_str).unwrap();

        //Save and return
        AlteredToml(data)
    }
}

/// Generate a randomised alphanumeric (base 62) string of a requested length.
pub fn generate_random_alphanumeric(length: usize) -> String {
    thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
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

impl Drop for AlteredToml {
    fn drop(&mut self) {
        std::fs::remove_file("./config/general-test.toml")
            .expect("unable to remove general-test.toml after test! Please delete manually");
    }
}