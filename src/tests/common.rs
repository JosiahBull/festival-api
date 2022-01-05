use std::collections::HashSet;
use std::path::PathBuf;

use crate::models::UserCredentials;
use crate::rocket;
use config::PathType;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::uri;
use utils::generate_random_alphanumeric;

//***** Helper Methods *****//
pub fn create_test_account(client: &Client) -> (UserCredentials, String, String) {
    let body = UserCredentials {
        usr: generate_random_alphanumeric(20),
        pwd: String::from("User12356789"),
    };
    let body_json = serde_json::to_string(&body).expect("a json body");

    //Create the account we wish to log into
    let create_response = client
        .post(uri!("/api/create"))
        .header(ContentType::new("application", "json"))
        .body(&body_json)
        .dispatch();
    assert_eq!(create_response.status(), Status::Created);

    (body, body_json, create_response.into_string().unwrap())
}

/// A simple struct which allows a property on toml to be changed.
pub struct AlteredToml(PathType, String);

impl AlteredToml {
    // TODO make this smarter by allowing a key-value replace, rather than a specific string.
    // This should make the test more robust.
    pub fn new(search: &str, replace: &str, p_type: PathType) -> Self {
        let path = p_type.get_path(&PathBuf::from("./config"));
        let data = std::fs::read_to_string(&path).unwrap();

        //Search through data
        let new_str = data.replace(search, replace);

        //Save data
        std::fs::write(&path, new_str).unwrap();

        //Save and return
        AlteredToml(p_type, data)
    }
}

impl Drop for AlteredToml {
    fn drop(&mut self) {
        let path = self.0.get_path(&PathBuf::from("./config"));
        std::fs::write(&path, &self.1).unwrap_or_else(|e| {
            panic!(
                "Unable to reset file {} after test due to error {}",
                path.to_string_lossy(),
                e
            )
        })
    }
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
