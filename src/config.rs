//! Configuration module for the api. This handles the loading, parsing, and updating of configuration options for the api.

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
};

use rocket::{
    fairing::{self, Fairing, Kind},
    request::{self, FromRequest},
    Build, Request, Rocket,
};

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
        Err(String::from("Key Not found in ./config/general.toml")) //FIXME
    };
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
    ($arg:tt, $type:ty) => {{
        fn load_val() -> $type {
            use crate::config::load_from_toml;
            use std::env::var;
            let env_name: &str = $arg;

            //1. Attempt to load from env
            //Attempt to load with truecase
            if let Ok(d) = var(env_name) {
                return d.parse().expect("a parsed value");
            }
            //Attempt to load with uppercase
            if let Ok(d) = var(env_name.to_uppercase()) {
                return d.parse().expect("a parsed value");
            }
            //Attempt to load with lowercase
            if let Ok(d) = var(env_name.to_lowercase()) {
                return d.parse().expect("a parsed value");
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
            panic!(
                "Env {} not found in environment or /config/general.toml. Program start failed.",
                env_name
            ); //FIXME
        }

        load_val()
    }};
    ($($arg:tt)*) => {
        compile_error!("Too many arguments provided to load_env macro!");
    };
}

fn load_supported_langs() -> HashMap<String, Language> {
    let file_path = PathType::Langs.get_path();
    let data = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Unable to find {}", file_path));
    let f = data
        .parse::<toml::Value>()
        .unwrap_or_else(|_| panic!("Unable to parse `{}`", file_path));

    let languages: &toml::value::Table = f
        .get("lang")
        .unwrap_or_else(|| panic!("Unable to parse {}, no langs provided!", file_path))
        .as_table()
        .unwrap_or_else(|| panic!("lang tag is not a table in {}", file_path));

    let mut map: HashMap<String, Language> = HashMap::default();
    let keys: Vec<&String> = languages.keys().into_iter().collect();
    for key in keys {
        let lang = languages
            .get(key)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse lang {} from {}, is it correctly formatted?",
                    key, file_path
                )
            })
            .as_table()
            .unwrap_or_else(|| panic!("Unable to prase {} as table from {}", key, file_path));

        let enabled = lang
            .get("enabled")
            .unwrap_or_else(|| panic!("Unable to parse enabled on {} from {}", key, file_path))
            .as_bool()
            .unwrap_or_else(|| panic!("{}'s enabled is not a boolean in {}", key, file_path));

        let festival_code = lang
            .get("festival_code")
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse festival_code on {} from {}",
                    key, file_path
                )
            })
            .as_str()
            .unwrap_or_else(|| panic!("{}'s festival_code is not a string in {}", key, file_path))
            .to_owned();

        let iso_691_code = lang
            .get("iso_691-1_code")
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse iso-691-1_code on {} from {}",
                    key, file_path
                )
            })
            .as_str()
            .unwrap_or_else(|| panic!("{}'s iso_691-1_code is not a string in {}", key, file_path))
            .to_owned();

        map.insert(
            iso_691_code.clone(),
            Language {
                display_name: key.clone(),
                enabled,
                festival_code,
                iso_691_code,
            },
        );
    }

    map
}

fn load_allowed_formats() -> HashSet<String> {
    let file_path = PathType::General.get_path();
    let data = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
    let f = data
        .parse::<toml::Value>()
        .unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

    let formats = table
        .get("ALLOWED_FORMATS")
        .unwrap_or_else(|| panic!("Unable to find ALLOWED_FORMATS in {}", file_path))
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "ALLOWED_FORMATS in {} is not an array of strings!",
                file_path
            )
        });

    let mut res = HashSet::default();

    for format in formats {
        let string = format
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "ALLOWED_FORMATS in {} is not an array of strings!",
                    file_path
                )
            })
            .to_owned();
        res.insert(string);
    }

    res
}

fn load_allowed_chars() -> HashSet<char> {
    let file_path = PathType::General.get_path();
    let data = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
    let f = data
        .parse::<toml::Value>()
        .unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

    let raw_string: String = table
        .get("ALLOWED_CHARS")
        .unwrap_or_else(|| panic!("Unable to find ALLOWED_CHARS in {}", file_path))
        .as_str()
        .unwrap_or_else(|| panic!("ALLOWED_CHARS in {} is not a string!", file_path))
        .to_owned();

    let mut res = HashSet::default();

    raw_string.chars().for_each(|c| {
        res.insert(c);
    });

    res
}

fn load_blacklisted_phrases() -> Vec<String> {
    let file_path = PathType::General.get_path();

    let data = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Unable to find `{}` due to error {}", file_path, e));
    let f = data
        .parse::<toml::Value>()
        .unwrap_or_else(|e| panic!("Unable to parse `{}` due to error {}", file_path, e));

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path));

    let phrases = table
        .get("BLACKLISTED_PHRASES")
        .unwrap_or_else(|| panic!("Unable to find BLACKLISTED_PHRASES in {}", file_path))
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "BLACKLISTED_PHRASES in {} is not an array of strings!",
                file_path
            )
        });

    let mut res = vec![];

    for phrase in phrases {
        let string = phrase
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "BLACKLISTED_PHRASES in {} is not an array of strings!",
                    file_path
                )
            })
            .to_owned();
        res.push(string);
    }

    res
}

pub struct Config {
    /// The secret used for fast-hashing JWT's for validation.
    jwt_secret: String,

    /// The number of hours that a JWT may be used before expiring and forcing the user to revalidate.
    jwt_expiry_time_hours: usize,

    /// The name of the api which is sent with certain requests.
    api_name: String,

    /// The path to the cache for storing .wav files.
    cache_path: String,

    /// The path where temporary files are stored, and should be deleted from on a crash.
    temp_path: String,

    /// The maximum length of a phrase that the api will process.
    word_length_limit: usize,

    /// The maximum speed at which a phrase can be read.
    speed_max_val: f32,

    /// The lowerest speed at which a phrase can be read.
    speed_min_val: f32,

    /// The maximum requests that an account can make in a given time period established by `MAX_REQUESTS_TIME_PERIOD_MINUTES`
    max_requests_acc_threshold: usize,

    /// The time period for timing out users who make too many requests.
    max_requests_time_period_minutes: usize,

    /// A list of supported speech languages by this api.
    supported_langs: HashMap<String, Language>,

    /// The list of supported file-formats, note that wav is the preferred format due to lower cpu usage.
    allowed_formats: HashSet<String>,

    /// A hashset of chars that the api will accept as input.
    allowed_chars: HashSet<char>,

    /// A list of phrases that are not allowed on this api.
    blacklisted_phrases: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            jwt_secret: load_env!("JWT_SECRET", String),
            jwt_expiry_time_hours: load_env!("JWT_EXPIRY_TIME_HOURS", usize),
            api_name: load_env!("API_NAME", String),
            cache_path: load_env!("CACHE_PATH", String),
            temp_path: load_env!("TEMP_PATH", String),
            word_length_limit: load_env!("WORD_LENGTH_LIMIT", usize),
            speed_max_val: load_env!("SPEED_MAX_VAL", f32),
            speed_min_val: load_env!("SPEED_MIN_VAL", f32),
            max_requests_acc_threshold: load_env!("MAX_REQUESTS_ACC_THRESHOLD", usize),
            max_requests_time_period_minutes: load_env!("MAX_REQUESTS_TIME_PERIOD_MINUTES", usize),
            supported_langs: load_supported_langs(),
            allowed_formats: load_allowed_formats(),
            allowed_chars: load_allowed_chars(),
            blacklisted_phrases: load_blacklisted_phrases(),
        }
    }
}

//TODO make a getter macro which can automatically generate all these
impl Config {
    pub fn JWT_SECRET(&self) -> &str {
        &self.jwt_secret
    }

    pub fn JWT_EXPIRY_TIME_HOURS(&self) -> usize {
        self.jwt_expiry_time_hours
    }

    pub fn API_NAME(&self) -> &str {
        &self.api_name
    }

    pub fn CACHE_PATH(&self) -> &str {
        &self.cache_path
    }

    pub fn TEMP_PATH(&self) -> &str {
        &self.temp_path
    }

    pub fn WORD_LENGTH_LIMIT(&self) -> usize {
        self.word_length_limit
    }

    pub fn SPEED_MAX_VAL(&self) -> f32 {
        self.speed_max_val
    }

    pub fn SPEED_MIN_VAL(&self) -> f32 {
        self.speed_min_val
    }

    pub fn MAX_REQUESTS_ACC_THRESHOLD(&self) -> usize {
        self.max_requests_acc_threshold
    }

    pub fn MAX_REQUESTS_TIME_PERIOD_MINUTES(&self) -> usize {
        self.max_requests_time_period_minutes
    }

    pub fn SUPPORTED_LANGS(&self) -> &HashMap<String, Language> {
        &self.supported_langs
    }

    pub fn ALLOWED_FORMATS(&self) -> &HashSet<String> {
        &self.allowed_formats
    }

    pub fn ALLOWED_CHARS(&self) -> &HashSet<char> {
        &self.allowed_chars
    }

    pub fn BLACKLISTED_PHRASES(&self) -> &[String] {
        &self.blacklisted_phrases
    }
}

impl Config {
    pub fn fairing() -> impl Fairing {
        Config::default()
    }
}

#[async_trait]
impl Fairing for Config {
    fn info(&self) -> rocket::fairing::Info {
        fairing::Info {
            name: "Custom Configuration Loader",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        //Generate Config
        let config = Config::default();

        //Save to State
        let new_rocket = rocket.manage(config);

        //Return our succesfully attached fairing!
        fairing::Result::Ok(new_rocket)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Config {
    type Error = Infallible;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Infallible> {
        let state = req.rocket().state::<Config>().unwrap();
        request::Outcome::Success(state)
    }
}

// #[cfg(test)]
// #[cfg(not(tarpaulin_include))]
// mod tests {
//     use lazy_static::lazy_static;
//     use load_env;

//     #[test]
//     #[should_panic]
//     fn test_failed_env_load() {
//         lazy_static! {
//             static ref U: String = load_env!("this_value_does_not_exist123");
//         }
//         lazy_static::initialize(&U);
//     }
// }
