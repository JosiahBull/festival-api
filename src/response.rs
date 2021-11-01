//! This module handles the generation of responses for the api.
//! 
//! 
//! Intended usage is to create a ResponseBuilder, then call.build on it
//! 
//! **Example:**
//! ```rust
//! 
//! #[get("/")]
//! fn my_endpoint<'a>() -> Response<'a> {
//!     //...processing...//
//!     
//!     ResponseBuilder {
//!         data: "A very well thought out, and meaningful response."
//!         status: rocket::http::Status::Ok
//!     }.build()
//! }
//! 
//! ```
//! 
//! This is a very helpful way to easily return responses to the user. Note that any type which implements Respondable
//! may be passed into the ResponseBuilder. This trait may also be implemented for your own types.
//! 
//! The trait must take a reference to your type, and then return a reference to a string which will be returned in
//! the body of the request to the user.
//! 
//! **Example:**
//! ```rust
//! impl Respondable for String {
//!     fn transform_body<'b>(&'b self) -> &'b str {
//!         self
//!     }
//!     fn transform_ct<'b>(&'b self) -> ContentType {
//!         ContentType::TextPlain
//!     }
//! } 
//! ```

use rocket::http::Status;
use rocket::request::Request;
use std::io::Cursor;

/// The different content-types which may be returned from this api. If providing an AudioMpeg filetype, you must also provide a filename.
#[derive(Debug, PartialEq)]
pub enum ContentType {
    JsonApplication,
    TextPlain,
    AudioMpeg(String),
}

/// Represents a response from the api, the content-type and content-disposition headers are automatically generated.
/// This is automatically generated from calling `.build()` on a `ResponseBuilder`. Do not attempt to generate this 
/// manually.
#[derive(Debug)]
pub struct Response {
    body: String,
    status: Status,
    c_type: ContentType,
}

/// Used to construct a `Response` to be returned by the api. Any type implementing `Respondable` may be passed into it.
/// Respondable is implemented for many default types, but you may implement it for your own types too.
pub struct ResponseBuilder<T>
where
    T: Respondable,
{
    pub data: T,
    pub status: Status,
}

impl<T> ResponseBuilder<T>
where
    T: Respondable,
{
    pub fn build(self) -> Response {
        let c_type = self.data.transform_ct();
        return match self.data.transform_body() {
            Ok(body) => {
                Response {
                    c_type,
                    body,
                    status: self.status,
                }       
            },
            Err(body) => {
                Response {
                    c_type: ContentType::TextPlain,
                    body,
                    status: Status::InternalServerError,
                }
            },
        }
    }
}

impl<T> Default for ResponseBuilder<T> 
where
    T: Respondable + Default,
{
    fn default() -> ResponseBuilder<T> {
        ResponseBuilder {
            data: T::default(),
            status: Status::Ok,
        }
    }
}

#[rocket::async_trait]
impl<'r> rocket::response::Responder<'r, 'static> for Response {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let body = self.body.to_owned(); //TODO find a way to avoid this alloc.

        //Generate content type header
        let c_type = match self.c_type {
            ContentType::JsonApplication => rocket::http::ContentType::new("application", "json"),
            ContentType::TextPlain => rocket::http::ContentType::new("text", "plain"),
            ContentType::AudioMpeg(_) => rocket::http::ContentType::new("audio", "mpeg"),
        };

        //Generate content disposition header
        let c_disp = match self.c_type {
            ContentType::AudioMpeg(s) => rocket::http::Header::new("Content-Disposition", format!("attachment; filename=\"{}\"", s)),
            _ => rocket::http::Header::new("Content-Disposition", "inline"),
        };

        //Construct and return response
        rocket::response::Response::build()
            .header(c_type)
            .header(c_disp)
            .status(self.status)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

/// A trait indicating that this datatype can be serialized into a response from this api.
pub trait Respondable {
    /// Generate the body of this response
    fn transform_body(self) -> Result<String, String>;
    /// Generate the content-type of this response.
    fn transform_ct<'a>(&'a self) -> ContentType;
}

impl<'a> Respondable for &'a str {
    fn transform_body(self) -> Result<String, String> {
        Ok(self.to_string())
    }
    fn transform_ct<'b>(&'b self) -> ContentType {
        ContentType::TextPlain
    }
}

impl Respondable for String {
    fn transform_body(self) -> Result<String, String> {
        Ok(self)
    }
    fn transform_ct<'b>(&'b self) -> ContentType {
        ContentType::TextPlain
    }
}

impl Respondable for crate::models::User {
    fn transform_body(self) -> Result<String, String> {
        serde_json::to_string(&self).map_err(|e| e.to_string())
    }

    fn transform_ct<'a>(&'a self) -> ContentType {
        ContentType::JsonApplication
    }
}

// TODO implement once https://github.com/rust-lang/rust/issues/84277 is stabilised
// Then all endpoints in `main.rs` can simply return Response, rather than Result<Response, Response>.
// impl<'a> std::ops::FromResidual<Result<std::convert::Infallible, response::Response<'_>>> for Response<'a> {
//     fn from_residual(residual: Result<std::convert::Infallible, response::Response<'_>>) -> Self {
//         todo!()
//     }
// }