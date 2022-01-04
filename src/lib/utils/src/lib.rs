mod utils {
    use rand::{thread_rng, Rng};
    use sha2::Digest;

    /// Generate a randomised alphanumeric (base 62) string of a requested length.
    pub fn generate_random_alphanumeric(length: usize) -> String {
        thread_rng()
            .sample_iter(rand::distributions::Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    /// Takes an input reference string, and hashes it using the sha512 algorithm.
    /// The resultant value is returned as a string in hexadecmial - meaning it is url and i/o safe.
    /// The choice of sha512 over sha256 is that sha512 tends to perform better at  longer strings - which we are likely to
    /// encounter with this api. Users the sha2 crate internally for hashing.
    pub fn sha_512_hash(input: &str) -> String {
        let mut hasher = sha2::Sha512::new();
        hasher.update(input);
        format!("{:x}", hasher.finalize())
    }
}

pub use utils::*;