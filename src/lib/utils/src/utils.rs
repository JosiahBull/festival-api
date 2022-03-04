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

/// Takes an input reference string, and hashes it using the sha256 algorithm.
/// The resultant value is returned as a string in hexadecmial - meaning it is url and i/o safe.
pub fn sha_256_hash(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::generate_random_alphanumeric;

    #[test]
    fn test_generate_random_alphanumeric() {
        //Note, there is a chance that we *could* get a string which has been generated before.
        //But that chance is infinitesimally small as to be negligible.
        let sample_size = 1000;
        let mut set: HashSet<String> = HashSet::default();
        for _ in 0..sample_size {
            let s = generate_random_alphanumeric(32);
            if set.contains(&s) {
                panic!("Duplicate key found in set");
            }
            set.insert(s);
        }
    }
}
