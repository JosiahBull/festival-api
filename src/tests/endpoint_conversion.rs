use crate::rocket;
use crate::{ALLOWED_FORMATS, BLACKLISTED_PHRASES, MAX_REQUESTS_ACC_THRESHOLD};
use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;
use rocket::uri;
use super::common::*;

#[test]
#[ignore]
fn blacklist_filter() {
    //HACK
    //Note that this test *must* run first. lazy_statics pollute the global environment when they run.
    //This means that if this test runs after another test which initalizes BLACKLISTED_PHRASES, it will fail.
    //I'm looking into a way to mitigate this -lazy_static- *really* shouldn't function this way in my opinion.
    //Note that this can be fixed by wrapping all config options in RwLocks. However that can happen when:

    let replace_search = "BLACKLISTED_PHRASES = []";
    let replace_data = "BLACKLISTED_PHRASES = [\"test\", \" things \", \" stuff \"]";
    let _t = AlteredToml::new(replace_search, replace_data);

    lazy_static::initialize(&BLACKLISTED_PHRASES);

    assert_eq!((*BLACKLISTED_PHRASES).len(), 3);

    let test_client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, _, token) = create_test_account(&test_client);

    //Begin Test
    //Simple test
    let body = "{
        \"word\": \"test\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token.clone()))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        "Blacklisted word! Phrase (test) is not allowed!"
    );

    //Test no spaces works
    let body = "{
        \"word\": \"adfadf-test-adfa\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token.clone()))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        "Blacklisted word! Phrase (test) is not allowed!"
    );

    //Check that no spaces works
    let body = "{
        \"word\": \"things\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token.clone()))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        "Blacklisted word! Phrase (things) is not allowed!"
    );

    //Check that spaces works
    let body = "{
        \"word\": \"vibes things my guy\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token.clone()))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        "Blacklisted word! Phrase (things) is not allowed!"
    );

    //Ensure multiple blocked words returns just the first
    let body = "{
        \"word\": \"testing things and stuff\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token.clone()))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::BadRequest);
    assert_eq!(
        response.into_string().unwrap(),
        "Blacklisted word! Phrase (test) is not allowed!"
    );
}

#[test]
fn success_conversion() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, _, token) = create_test_account(&client);

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    //Test the generation of the .wav file
    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token))
        .body(&body)
        .dispatch();

    let status = response.status();
    if status != Status::Ok {
        panic!(
            "Failed with status {} \nBody: \n{}\n",
            status,
            response.into_string().unwrap()
        );
    }

    assert_eq!(
        response.headers().get_one("content-type").unwrap(),
        "audio/mpeg"
    );

    assert_eq!(
        response.headers().get_one("content-disposition").unwrap(),
        "attachment; filename=\"output.wav\""
    );
    assert_eq!(response.into_bytes().unwrap().len(), 63840);
}

#[test]
fn invalid_conversion_strings() {
    //List of potentially "invalid" phrases to test
    //When the sytem tries to create the file on the disk
    //Note that we are *not* testing that the api rejects these strings, we are only testing
    //That they don't cause a panic and we get a reasonable response (i.e. not 500)
    let dangerous_phrases: [&str; 15] = [
        "\\\\\\//////",
        "\\",
        ".",
        ".mp4",
        "something.png",
        "\0",
        "\0\0\00\0\\\\\\////\\\\/\\\0\0\0\0",
        ">",
        ">><<",
        "|",
        "||",
        ":",
        "&",
        "&&",
        "::",
    ];
    for phrase in dangerous_phrases {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);
        let body = format!(
            "{{
            \"word\": \"{}\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"wav\"
        }}",
            phrase
        );

        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token.clone()))
            .body(&body)
            .dispatch();

        assert_ne!(response.status(), Status::InternalServerError);
    }
}

#[test]
fn test_limits() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, _, token) = create_test_account(&client);

    let body = format!(
        "{{
        \"word\": \"The University of Auckland_{}\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }}",
        generate_random_alphanumeric(5)
    );

    for _ in 0..*MAX_REQUESTS_ACC_THRESHOLD {
        //Test the generation of the .wav file
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token.clone()))
            .body(&body)
            .dispatch();
        let status = response.status();
        if status != Status::Ok {
            panic!(
                "Failed with status {} \nBody: \n{}\n",
                status,
                response.into_string().unwrap()
            );
        }
        assert_eq!(status, Status::Ok);
    }

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token))
        .body(&body)
        .dispatch();

    let status = response.status();
    if status != Status::TooManyRequests {
        panic!(
            "Failed with status {} \nBody: \n{}\n",
            status,
            response.into_string().unwrap()
        );
    }

    //TODO this could check for a tolerance on the seconds number?
    assert!(response
        .into_string()
        .unwrap()
        .contains("Too many requests! You will be able to make another request in"));
}

/// Validate that all file format options work as intended
#[test]
fn test_every_format() {
    for format in ALLOWED_FORMATS.iter() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let (_, _, token) = create_test_account(&client);

        let body = format!(
            "{{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"{}\"
        }}",
            format
        );

        //Generate a 'generic' file and validate the response is correct
        let response = client
            .post(uri!("/api/v1/convert"))
            .header(ContentType::new("application", "json"))
            .header(Header::new("Authorisation", token))
            .body(&body)
            .dispatch();

        let status = response.status();
        if status != Status::Ok {
            panic!(
                "Failed with status {} \nBody: \n{}\n",
                status,
                response.into_string().unwrap()
            );
        }
        let expected = format!("attachment; filename=\"output.{}\"", format);
        assert_eq!(
            response.headers().get_one("content-disposition").unwrap(),
            expected
        );
    }
}

/// Validate that all languages are accepted by the api
#[test]
fn test_every_lang() {
    //TODO
}

/// Ensure that incorrect tokens fail as expected
#[test]
fn test_invalid_auth_tokens() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, _, token) = create_test_account(&client);

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    //Test No Header
    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::Unauthorized);
    assert_eq!(
        response.into_string().unwrap(),
        "Authorisation Header Not Present"
    );

    //Test Invalid Token

    let bad_token = format!("a{}", &token);

    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", bad_token))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::Unauthorized);
    assert_eq!(response.into_string().unwrap(), "Invalid Auth Token");

    //Test Invalid Header
    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorizationadf", token))
        .body(&body)
        .dispatch();

    assert_eq!(response.status(), Status::Unauthorized);
    assert_eq!(
        response.into_string().unwrap(),
        "Authorisation Header Not Present"
    );
}

/// A simple test which ensures that an invalid file format fails out as expected.
#[test]
fn test_invalid_formats() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let (_, _, token) = create_test_account(&client);

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"this-will-never-exist\"
    }";

    //Generate a 'generic' file and validate the response is correct
    let response = client
        .post(uri!("/api/v1/convert"))
        .header(ContentType::new("application", "json"))
        .header(Header::new("Authorisation", token))
        .body(&body)
        .dispatch();

    let status = response.status();
    if status != Status::BadRequest {
        panic!(
            "Failed with status {} \nBody: \n{}\n",
            status,
            response.into_string().unwrap()
        );
    }

    let body = response.into_string().expect("a valid body");
    assert_eq!(
        body,
        String::from("Requested format (this-will-never-exist) is not supported by this api!")
    );
}