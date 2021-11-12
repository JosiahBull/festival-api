/// A struct which represents settings for a specific user.
/// This is loaded on boot from `./config/users.toml`.
#[derive(Debug, Clone)]
pub struct UserSettings {
    pub apply_api_rate_limit: bool,
}

/// Represents a possible language that the api may convert text into.
/// This is loaded on boot from `./config/langs.toml`.
#[derive(Debug, Clone)]
pub struct Language {
    pub display_name: String,
    pub iso_691_code: String,
    pub festival_code: String,
    pub enabled: bool,
}

//TODO implement display for both types
