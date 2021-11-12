mod response;
pub use crate::response::*;

#[cfg(not(tarpaulin_include))]
#[cfg(test)]
mod tests {
    use rocket::http::Status;

    use super::{Data, Response};

    #[test]
    fn responder_basics() {
        let data: Data<String> = Data {
            data: String::from("hello, world"),
            status: Status::Ok,
        };

        let response = Response::TextOk(data);

        match response {
            Response::TextOk(s) => {
                assert_eq!(s.status(), Status::Ok);
                assert_eq!(*s.data(), String::from("hello, world"));
            }
            _ => panic!("Invalid type!"),
        }
    }
}
