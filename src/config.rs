//! Configuration module for the api. This handles the loading, parsing, and updating of configuration options for the api.

use std::collections::{HashMap, HashSet};
use lazy_static::lazy_static;

use crate::models::Language;

//General Todos
//TODO: Macroise a lot of the initalisation code to clean it up.
//TODO: Create functions that load HashSet<T>, or Vec<T> types from toml - this should cleanup the code nicely.
//TODO: Work on a viable method for dynamically loading the settings (RwLock) seems most promising - as shown
// at the bottom of this page

/// The location of all config files on the system.
const CONFIG_LOCATION: &str = "./config/";

/// The different config paths we can load from
enum PathType {
    General,
    Langs,
    // Users,
}

impl PathType {
    fn get_path(&self) -> String {
        let name = match *self {
            PathType::General => "general",
            PathType::Langs => "langs",
            // PathType::Users => "users",
        };

        let testing_file = format!("{}/{}-test.toml", CONFIG_LOCATION, name);
        if std::path::Path::new(&testing_file).exists() {
            testing_file
        } else {
            format!("{}/{}.toml", CONFIG_LOCATION, name)
        }
    }
}

/// Opens a toml file, and attempts to load the toml::value as specified in the provided &str.
fn load_from_toml(name: &str) -> Result<toml::Value, String> {
    let file_path = PathType::General.get_path();
    let data = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let f = data.parse::<toml::Value>().map_err(|e| e.to_string())?;

    return if let Some(k) = f.get(name) {
        Ok(k.to_owned())
    } else {
        Err(String::from("Key Not found in ./config/general.toml"))
    }
}

/// A macro to load configuration from the environment.
///
/// Attempts to load from multiple sources falling back in this order:
/// 1. Load from environment
/// 2. Load from `./config/general.toml`
/// 3. panic!
///
/// This macro recommend for use in conjunction with lazy static, as these variables like to be loaded/parsed
/// at runtime, not at compile-time.
///
/// **Example**
/// ```rust
///     lazy_static! {
///         static ref NUMBER_SHOES: usize = load_env!("NUMBER_SHOES");
///     }
///
///     lazy_static::initialize(&NUMBER_SHOES);
///     println!("The number of shoes is {}", *NUMBER_SHOES);
///     / ```
/// A vay of types are supported for implicit conversion, look [here](https://docs.rs/toml/0.5.8/toml/value/enum.Value.html#impl-From%3C%26%27a%20str%3E) for a dedicated list of these types.
///
/// Internally this macro relies on `toml::value::Value.try_into()` for type conversion.
///
macro_rules! load_env {
    () => {
        compile_error!("String must be provided to load_env macro!");
    };
    ($arg:tt) => {
        {
            use std::env::var;
            use crate::config::load_from_toml;
            let env_name: &str = $arg;

            //1. Attempt to load from env
            //Attempt to load with truecase
            if let Ok(d) = var(env_name) {
                return d.parse().expect("a parsed value")
            }
            //Attempt to load with uppercase
            if let Ok(d) = var(env_name.to_uppercase()) {
                return d.parse().expect("a parsed value")
            }
            //Attempt to load with lowercase
            if let Ok(d) = var(env_name.to_lowercase()) {
                return d.parse().expect("a parsed value")
            }

            //2. Attempt to load from /config/general.toml
            //Attempt to load with truecase
            if let Ok(d) = load_from_toml(&env_name) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }
            //Attempt to load with uppercase
            if let Ok(d) = load_from_toml(&env_name.to_uppercase()) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }
            //Attempt to load lowercase
            if let Ok(d) = load_from_toml(&env_name.to_lowercase()) {
                if let Ok(v) = d.try_into() {
                    return v;
                }
            }

            //3. Failure
            panic!("Env {} not found in environment, ./.env or /config/general.toml. Program start failed.", env_name)
        }
    };
    ($($arg:tt)*) => {
        compile_error!("Too many arguments provided to load_env macro!");
    };
}

lazy_static! {
    /// The secret used for fast-hashing JWT's for validation.
    pub static ref JWT_SECRET: String = load_env!("JWT_SECRET");
    
    /// The number of hours that a JWT may be used before expiring and forcing the user to revalidate.
    pub static ref JWT_EXPIRY_TIME_HOURS: usize = load_env!("JWT_EXPIRY_TIME_HOURS");
    
    /// The name of the api which is sent with certain requests.
    pub static ref API_NAME: String = load_env!("API_NAME");
    
    /// The path to the cache for storing .wav files.
    pub static ref CACHE_PATH: String = load_env!("CACHE_PATH");
    
    /// The path where temporary files are stored, and should be deleted from on a crash.
    pub static ref TEMP_PATH: String = load_env!("TEMP_PATH");
    
    /// The maximum length of a phrase that the api will process.
    pub static ref WORD_LENGTH_LIMIT: usize = load_env!("WORD_LENGTH_LIMIT");
    
    /// The maximum speed at which a phrase can be read.
    pub static ref SPEED_MAX_VAL: f32 = load_env!("SPEED_MAX_VAL");
    
    /// The lowerest speed at which a phrase can be read.
    pub static ref SPEED_MIN_VAL: f32 = load_env!("SPEED_MIN_VAL");
    
    /// The maximum requests that an account can make in a given time period established by `MAX_REQUESTS_TIME_PERIOD_MINUTES`
    pub static ref MAX_REQUESTS_ACC_THRESHOLD: usize = load_env!("MAX_REQUESTS_ACC_THRESHOLD");
    
    /// The time period for timing out users who make too many requests.
    pub static ref MAX_REQUESTS_TIME_PERIOD_MINUTES:usize = load_env!("MAX_REQUESTS_TIME_PERIOD_MINUTES");

    /// A list of supported speech languages by this api.
    pub static ref SUPPORTED_LANGS: HashMap<String, Language> = {
        let file_path = PathType::Langs.get_path();
        let data = std::fs::read_to_string(&file_path).unwrap_or_else(|_| panic!("Unable to find {}", file_path));
        let f = data.parse::<toml::Value>().unwrap_or_else(|_| panic!("Unable to parse `{}`", file_path));

        let languages: &toml::value::Table = f.get("lang")
            .unwrap_or_else(|| panic!("Unable to parse {}, no langs provided!", file_path))
            .as_table()
            .unwrap_or_else(|| panic!("lang tag is not a table in {}", file_path));

        let mut map: HashMap<String, Language> = HashMap::default();
        let keys: Vec<&String> = languages.keys().into_iter().collect();
        for key in keys {
            let lang = languages
                .get(key)
                .unwrap_or_else(|| panic!("Unable to parse lang {} from {}, is it correctly formatted?", key, file_path))
                .as_table()
                .unwrap_or_else(|| panic!("Unable to prase {} as table from {}", key, file_path));

            let enabled = lang
                .get("enabled")
                .unwrap_or_else(|| panic!("Unable to parse enabled on {} from {}", key, file_path))
                .as_bool()
                .unwrap_or_else(|| panic!("{}'s enabled is not a boolean in {}", key, file_path));

            let festival_code = lang
                .get("festival_code")
                .unwrap_or_else(|| panic!("Unable to parse festival_code on {} from {}", key, file_path))
                .as_str()
                .unwrap_or_else(|| panic!("{}'s festival_code is not a string in {}", key, file_path))
                .to_owned();

            let iso_691_code = lang
                .get("iso_691-1_code")
                .unwrap_or_else(|| panic!("Unable to parse iso-691-1_code on {} from {}", key, file_path))
                .as_str()
                .unwrap_or_else(|| panic!("{}'s iso_691-1_code is not a string in {}", key, file_path))
                .to_owned();

            map.insert(iso_691_code.clone(), Language {
                display_name: key.clone(),
                enabled,
                festival_code,
                iso_691_code,
            });
        }

        map
    };

    /// The list of supported file-formats, note that wav is the preferred format due to lower cpu usage.
    pub static ref ALLOWED_FORMATS: HashSet<String> = {
        let file_path = PathType::General.get_path();
        let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
        let f = data.parse::<toml::Value>().unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

        let table = f.as_table().unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

        let formats = table.get("ALLOWED_FORMATS")
            .unwrap_or_else(|| panic!("Unable to find ALLOWED_FORMATS in {}", file_path))
            .as_array()
            .unwrap_or_else(|| panic!("ALLOWED_FORMATS in {} is not an array of strings!", file_path));

        let mut res = HashSet::default();

        for format in formats {
            let string = format
                .as_str()
                .unwrap_or_else(|| panic!("ALLOWED_FORMATS in {} is not an array of strings!", file_path))
                .to_owned();
            res.insert(string);
        }

        res
    };

    /// A hashset of chars that the api will accept as input.
    pub static ref ALLOWED_CHARS: HashSet<char> = {
        let file_path = PathType::General.get_path();
        let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
        let f = data.parse::<toml::Value>().unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

        let table = f.as_table().unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

        let raw_string: String = table.get("ALLOWED_CHARS")
            .unwrap_or_else(|| panic!("Unable to find ALLOWED_CHARS in {}", file_path))
            .as_str()
            .unwrap_or_else(|| panic!("ALLOWED_CHARS in {} is not a string!", file_path))
            .to_owned();

        let mut res = HashSet::default();

        raw_string.chars().for_each(|c| {
            res.insert(c);
        });

        res
    };

    /// A list of phrases that are not allowed on this api.
    pub static ref BLACKLISTED_PHRASES: Vec<String> = {
        let file_path = PathType::General.get_path();

        let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
        let f = data.parse::<toml::Value>().unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

        let table = f.as_table().unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

        let phrases = table.get("BLACKLISTED_PHRASES")
            .unwrap_or_else(|| panic!("Unable to find BLACKLISTED_PHRASES in {}", file_path))
            .as_array()
            .unwrap_or_else(|| panic!("BLACKLISTED_PHRASES in {} is not an array of strings!", file_path));

        let mut res = vec![];

        for phrase in phrases {
            let string = phrase
                .as_str()
                .unwrap_or_else(|| panic!("BLACKLISTED_PHRASES in {} is not an array of strings!", file_path))
                .to_owned();
            res.push(string);
        }

        res
    };
}

pub fn initalize_globals() {
    lazy_static::initialize(&JWT_SECRET);
    lazy_static::initialize(&JWT_EXPIRY_TIME_HOURS);
    lazy_static::initialize(&API_NAME);
    lazy_static::initialize(&CACHE_PATH);
    lazy_static::initialize(&TEMP_PATH);
    lazy_static::initialize(&SPEED_MAX_VAL);
    lazy_static::initialize(&SPEED_MIN_VAL);
    lazy_static::initialize(&MAX_REQUESTS_ACC_THRESHOLD);
    lazy_static::initialize(&MAX_REQUESTS_TIME_PERIOD_MINUTES);
    lazy_static::initialize(&SUPPORTED_LANGS);
    lazy_static::initialize(&ALLOWED_FORMATS);
    lazy_static::initialize(&ALLOWED_CHARS);
    lazy_static::initialize(&BLACKLISTED_PHRASES);
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use load_env;
    use lazy_static::lazy_static;

    #[test]
    #[should_panic]
    fn test_failed_env_load() {
        lazy_static! {
            static ref U: String = load_env!("this_value_does_not_exist123");
        }
        lazy_static::initialize(&U);
    }
}


// This could be a solid implementation to allow general editing of these configuration options.
// This would only really be useful for testing purposes though... so is it worth the overhead?

// struct ConfigItem<T> {
//     data: RwLock<T>
// }

// impl<T> ConfigItem<T> 
// where
//     T: Clone,
// {
//     async fn set(&self, s: T) {
//         let mut d = self.data.write().await;
//         *d = s;
//     }

//     async fn get(&self) -> &T {
//         self.data.read().await.as_ref()
//     }
// }

// lazy_static! {
//     static ref TEST: ConfigItem<String> = {
//         ConfigItem {
//             data: RwLock::new(String::from("hello"))
//         }
//     };
// }

// async fn do_things() {
//     TEST.set(String::from("hello")).await;

// }