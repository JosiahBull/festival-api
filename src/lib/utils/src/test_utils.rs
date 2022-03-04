use config::PathType;
use std::path::PathBuf;

//***** Helper Methods *****//
/// A simple struct which allows a property on toml to be changed.
pub struct AlteredToml(PathType, String, PathBuf);

#[allow(dead_code)]
impl AlteredToml {
    // TODO make this smarter by allowing a key-value replace, rather than a specific string.
    // This should make the test more robust.
    pub fn new(search: &str, replace: &str, p_type: PathType, replace_path: PathBuf) -> Self {
        let path = p_type.get_path(&replace_path);
        let data = std::fs::read_to_string(&path).unwrap();

        //Search through data
        let new_str = data.replace(search, replace);

        //Save data
        std::fs::write(&path, new_str).unwrap();

        //Save and return
        AlteredToml(p_type, data, replace_path)
    }
}

impl Drop for AlteredToml {
    fn drop(&mut self) {
        let path = self.0.get_path(&self.2);
        std::fs::write(&path, &self.1).unwrap_or_else(|e| {
            panic!(
                "Unable to reset file {} after test due to error {}",
                path.to_string_lossy(),
                e
            )
        })
    }
}
