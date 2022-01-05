use crate::rocket;
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::uri;

//***** Test Methods *****//

#[test]
fn test_index() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!(crate::index)).dispatch();
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
