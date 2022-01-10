use festival_api::models::UserCredentials;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::uri;
use utils::generate_random_alphanumeric;

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