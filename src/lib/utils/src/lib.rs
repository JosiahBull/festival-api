mod utils {
    use std::{path::{Path, PathBuf}, ops::Deref, fmt::Debug};

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

    /// A useful file handle for wrapping temporary files, without any form of validation or checking
    #[derive(Debug, Clone)]
    pub struct FileHandle {
        path: PathBuf,
        to_cache: bool,
    }

    impl std::fmt::Display for FileHandle {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.path.fmt(f)
        }
    }

    impl FileHandle {
        pub fn new(path: PathBuf, to_cache: bool,) -> Self {
            Self { path, to_cache }
        }

        pub fn underlying(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for FileHandle {
        #[allow(unused_must_use)]
        fn drop(&mut self) {
            if !self.to_cache {
                std::fs::remove_file(Path::new(&self.path));
            }
        }
    }

    impl Deref for FileHandle {
        type Target = PathBuf;

        fn deref(&self) -> &Self::Target {
            &self.path
        }
    }
}

pub use utils::*;
