use config::{Config, PathType};
use festival_api::rocket;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use rocket::uri;
use std::path::PathBuf;
use utils::test_utils::AlteredToml;

/// Test that the word blacklist works correctly
#[test]
fn blacklist_filter() {
    let replace_search = "BLACKLISTED_PHRASES = []";
    let replace_data = "BLACKLISTED_PHRASES = [\"test\", \" things \", \" stuff \"]";
    let _t = AlteredToml::new(
        replace_search,
        replace_data,
        PathType::General,
        PathBuf::from("./config"),
    );

    let test_client = Client::tracked(rocket()).expect("valid rocket instance");

    //Begin Test
    //Simple test
    let body = "{
        \"word\": \"test\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    let response = test_client
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"wav\"
    }";

    //Test the generation of the .wav file
    let response = client
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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
    assert!(response.into_bytes().unwrap().len() > 30000);
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
            .post(uri!("/api/convert"))
            .header(ContentType::new("application", "json"))
            .body(&body)
            .dispatch();

        assert_ne!(response.status(), Status::InternalServerError);
    }
}

/// Validate that all file format options work as intended
#[test]
fn test_every_format() {
    let cfg: Config = Config::new(PathBuf::from("./config")).unwrap();
    for format in cfg.ALLOWED_FORMATS().iter() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");

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
            .post(uri!("/api/convert"))
            .header(ContentType::new("application", "json"))
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

/// A simple test which ensures that an invalid file format fails out as expected.
#[test]
fn test_invalid_formats() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");

    let body = "{
        \"word\": \"The University of Auckland\",
        \"lang\": \"en\",
        \"speed\": 1.0,
        \"fmt\": \"this-will-never-exist\"
    }";

    //Generate a 'generic' file and validate the response is correct
    let response = client
        .post(uri!("/api/convert"))
        .header(ContentType::new("application", "json"))
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

#[test]
fn test_speed() {
    let cfg: Config = Config::new(PathBuf::from("./config")).unwrap();

    for format in cfg.ALLOWED_FORMATS().iter() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");

        let normal = format!(
            "{{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 1.0,
            \"fmt\": \"{}\"
        }}",
            format
        );
        let fast = format!(
            "{{
            \"word\": \"The University of Auckland\",
            \"lang\": \"en\",
            \"speed\": 2.0,
            \"fmt\": \"{}\"
        }}",
            format
        );

        let response_normal = client
            .post(uri!("/api/convert"))
            .header(ContentType::new("application", "json"))
            .body(&normal)
            .dispatch();

        let response_fast = client
            .post(uri!("/api/convert"))
            .header(ContentType::new("application", "json"))
            .body(&fast)
            .dispatch();

        let normal_size = response_normal.into_bytes().unwrap().len();
        let fast_size = response_fast.into_bytes().unwrap().len();

        let diff = (fast_size / normal_size) as f64 - 0.5;

        println!(
            "Testing format {} with diff: {}\nnorm:{} fast:{}",
            format, diff, normal_size, fast_size
        );

        assert!(diff < 0.05);
    }
}
