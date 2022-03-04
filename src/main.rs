#![doc = include_str!("../readme.md")]

pub mod models;

#[macro_use]
extern crate rocket;

use cache_manager::Cache;
use config::Config;
use converter::{Converter, Ffmpeg};
use festvox::{Flite, TtsGenerator};
use macros::failure;
use response::{Data, Response};
use rocket::{fs::NamedFile, http::Status, serde::json::Json};
use utils::phrase_package::PhrasePackage;

#[cfg(not(target_os = "linux"))]
compile_error!("Unable to compile for your platform! This API is only available for Linux due to dependence on Bash commands.");

/// The base url of the program. This is just a catch-all for those who stumble across the api without knowing what it does.
#[get("/")]
pub fn index(cfg: &Config) -> String {
    format!("Welcome to {}'s TTS API.", cfg.API_NAME())
}

/// Expects a phrase package, attempts to convert it to a sound file to be returned to the user.
/// Requires an authenticate user account to access. This endpoint also features strict rate limiting
/// as generating .wav files is very resource intensive.
#[post("/convert", data = "<phrase_package>", format = "application/json")]
pub async fn convert(
    mut phrase_package: Json<PhrasePackage>,
    generator: &Flite,
    converter: &Converter,
    cfg: &Config,
    cache: Cache,
) -> Result<Response, Response> {
    // Validate PhrasePackage
    phrase_package.validated(cfg).map_err(|e| {
        Response::TextErr(Data {
            data: e,
            status: Status::BadRequest,
        })
    })?;
    let phrase_package = phrase_package.into_inner();

    // Generate the phrase
    let generated_file = generator
        .generate(&phrase_package, cfg)
        .await
        .map_err(|e| {
            error!("{e}");
            Response::TextErr(Data {
                data: String::from(
                    "an error occured in festival/flite while generating the requested phrase",
                ),
                status: Status::InternalServerError,
            })
        })?;

    // Convert the file
    if !converter.is_supported(&phrase_package.fmt) {
        failure!("requested file format is not available")
    }

    //Generate Response
    let response = match converter.convert(
        &phrase_package,
        phrase_package.speed,
        cfg,
    ).await {
        Ok(f) => {
            let resp_file = match NamedFile::open(f).await {
                Ok(f) => f,
                Err(e) => failure!("Unable to open processed file {}, this is an internal error", e),
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
    };

    //Cache File
    if let Err(e) = cache.used(generated_file.to_path_buf()).await {
        error!("cache error {}", e);
        failure!("cache failure");
    }

    response
}

#[doc(hidden)]
#[launch]
pub fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/api/", routes![convert])
        .attach(Config::fairing())
        .attach(Flite::fairing())
        .attach(Converter::fairing(vec![Box::new(
            Ffmpeg::new().expect("a valid ffmpeg instance"),
        )]))
        .attach(Cache::fairing())
}
