//! Various objects, including database objects, for the api.
use chrono::Utc;

/// A request to generate a .wav file from text from a user that has been stored in the db.
/// This is a return object from the reqs table of the database.
pub struct GenerationRequest {
    pub id: i32,
    pub usr_id: i32,
    pub crt: chrono::DateTime<Utc>,
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}

/// A request to generate a .wav file from text for a user, to be stored in the db.
/// This is an insertion object for the reqs table of the database.
pub struct NewGenerationRequest {
    pub usr_id: i32,
    pub word: String,
    pub lang: String,
    pub speed: f32,
    pub fmt: String,
}