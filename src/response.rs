use rocket::{fs::NamedFile, http::Status, response::Responder, Request};

#[derive(Debug)]
pub struct Data<T>
where
    T: Responder<'static, 'static>,
{
    pub data: T,
    pub status: Status,
}

/// Represents a response from the api, the content-type and content-disposition headers are automatically generated.
/// This is automatically generated from calling `.build()` on a `ResponseBuilder`. Do not attempt to generate this
/// manually.
#[derive(Debug)]
pub enum Response {
    TextErr(Data<String>),
    TextOk(Data<String>),
    #[allow(dead_code)]
    JsonOk(Data<String>),
    FileDownload(Data<NamedFile>),
}

#[rocket::async_trait]
impl<'r> rocket::response::Responder<'r, 'static> for Response {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        //Generate content type header
        let c_type = match self {
            Response::TextErr(_) | Response::TextOk(_) => {
                rocket::http::ContentType::new("text", "plain; charset=utf-8")
            }
            Response::JsonOk(_) => {
                rocket::http::ContentType::new("application", "json; charset=utf-8")
            }
            Response::FileDownload(_) => rocket::http::ContentType::new("audio", "mpeg"),
        };

        //Generate content disposition header
        let c_disp = match self {
            Response::FileDownload(ref d) => rocket::http::Header::new(
                "Content-Disposition",
                format!(
                    "attachment; filename=\"{}\"",
                    d.data.path().to_string_lossy()
                ),
            ),
            _ => rocket::http::Header::new("Content-Disposition", "inline"),
        };

        let status: Status = match self {
            Response::TextErr(ref d) => d.status,
            Response::TextOk(ref d) => d.status,
            Response::JsonOk(ref d) => d.status,
            Response::FileDownload(ref d) => d.status,
        };

        //Construct and return response
        let response = match self {
            Response::TextErr(d) => d.data.respond_to(req),
            Response::TextOk(d) => d.data.respond_to(req),
            Response::JsonOk(d) => d.data.respond_to(req),
            Response::FileDownload(d) => d.data.respond_to(req),
        };

        let mut response = response.unwrap(); //HACK

        response.set_header(c_type);
        response.set_header(c_disp);
        response.set_status(status);
        Ok(response)
    }
}

// TODO implement once https://github.com/rust-lang/rust/issues/84277 is stabilised
// Then all endpoints in `main.rs` can simply return Response, rather than Result<Response, Response>.
// impl<'a> std::ops::FromResidual<Result<std::convert::Infallible, response::Response<'_>>> for Response<'a> {
//     fn from_residual(residual: Result<std::convert::Infallible, response::Response<'_>>) -> Self {
//         todo!()
//     }
// }
