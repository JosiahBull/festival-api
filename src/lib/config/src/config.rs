//! Configuration module for the api. This handles the loading, parsing, and updating of configuration options for the api.

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    env::var,
    path::PathBuf,
    str::FromStr,
};

use rocket::{
    fairing::AdHoc,
    request::{self, FromRequest},
    Request,
};

use crate::error::ConfigError;
use crate::models::{Language};

//General Todos
//TODO: Macroise a lot of the initalisation code to clean it up.
//TODO: Create functions that load HashSet<T>, or Vec<T> types from toml - this should cleanup the code nicely.
//TODO: Work on a viable method for dynamically loading the settings (RwLock) seems most promising - as shown
// at the bottom of this page

/// The different config paths we can load from
pub enum PathType {
    General,
    Langs,
    Users,
}

impl PathType {
    pub fn get_path(&self, config_path: &PathBuf) -> PathBuf {
        let name = match *self {
            PathType::General => "general",
            PathType::Langs => "langs",
            PathType::Users => "users",
        };

        config_path.join(format!("{}.toml", name))
    }
}

/// Opens a toml file, and attempts to load the toml::value as specified in the provided &str.
fn load_from_toml(name: &str, path: &PathBuf) -> Result<toml::Value, String> {
    let file_path = PathType::General.get_path(path);
    let data = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let f = data.parse::<toml::Value>().map_err(|e| e.to_string())?;

    return if let Some(k) = f.get(name) {
        Ok(k.to_owned())
    } else {
        Err(format!("Key Not found in {}", file_path.to_string_lossy()))
    };
}

/// A function to load configuration from the environment.
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
/// ```ignore
///     let num_shoes: usize = load_env("&path");
///     assert_eq!(num_shoes, 5);
///     println!("The number of shoes is {}", num_shoes);
/// ```
///
/// A variety of types are supported for implicit conversion, look [here](https://docs.rs/toml/0.5.8/toml/value/enum.Value.html#impl-From%3C%26%27a%20str%3E) for a dedicated list of these types.
///
/// Internally this macro relies on 'toml::value::Value.try_into()' for type conversion.
///
fn load_env<T, E>(env_name: &str, path: &PathBuf) -> Result<T, ConfigError>
where
    T: FromStr + FromStr<Err = E> + rocket::serde::Deserialize<'static>,
    E: Into<ConfigError>,
{
    //1. Attempt to load from env
    if let Ok(val) = var(env_name) {
        return Ok(val.parse().map_err(|e: E| {
            let e: ConfigError = e.into();
            e
        })?);
    }

    //2. Attempt to load from config location
    if let Ok(val) = load_from_toml(env_name, path) {
        if let Ok(val) = val.try_into() {
            return Ok(val);
        }
    }

    panic!(
        "Env {} not found in environment or {}. Program start failed.",
        env_name,
        PathType::General.get_path(path).to_string_lossy()
    )
}

fn load_table(file_path: &PathBuf, table_name: &str) -> Result<toml::value::Table, ConfigError> {
    let data = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Unable to find {}", file_path.to_string_lossy()));
    let f = data
        .parse::<toml::Value>()
        .unwrap_or_else(|_| panic!("Unable to parse `{}`", file_path.to_string_lossy()));

    let table: toml::value::Table = f
        .get(table_name)
        .unwrap_or_else(|| {
            panic!(
                "Unable to parse {}, no langs provided!",
                file_path.to_string_lossy()
            )
        })
        .as_table()
        .unwrap_or_else(|| panic!("lang tag is not a table in {}", file_path.to_string_lossy()))
        .to_owned();

    Ok(table)
}

fn load_supported_langs(path: &PathBuf) -> Result<HashMap<String, Language>, ConfigError> {
    let file_path = PathType::Langs.get_path(path);
    let languages = load_table(&file_path, "lang")?;

    let mut map: HashMap<String, Language> = HashMap::default();
    let keys: Vec<&String> = languages.keys().into_iter().collect();
    for key in keys {
        let lang = languages
            .get(key)
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse lang {} from {}, is it correctly formatted?",
                    key,
                    file_path.to_string_lossy()
                )
            })
            .as_table()
            .unwrap_or_else(|| {
                panic!(
                    "Unable to prase {} as table from {}",
                    key,
                    file_path.to_string_lossy()
                )
            });

        let enabled = lang
            .get("enabled")
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse enabled on {} from {}",
                    key,
                    file_path.to_string_lossy()
                )
            })
            .as_bool()
            .unwrap_or_else(|| {
                panic!(
                    "{}'s enabled is not a boolean in {}",
                    key,
                    file_path.to_string_lossy()
                )
            });

        let festival_code = lang
            .get("festvox")
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse festival_code on {} from {}",
                    key,
                    file_path.to_string_lossy()
                )
            })
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "{}'s festival_code is not a string in {}",
                    key,
                    file_path.to_string_lossy()
                )
            })
            .to_owned();

        let iso_691_code = lang
            .get("iso_691-1_code")
            .unwrap_or_else(|| {
                panic!(
                    "Unable to parse iso-691-1_code on {} from {}",
                    key,
                    file_path.to_string_lossy()
                )
            })
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "{}'s iso_691-1_code is not a string in {}",
                    key,
                    file_path.to_string_lossy()
                )
            })
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

    Ok(map)
}

fn load_allowed_formats(path: &PathBuf) -> Result<HashSet<String>, ConfigError> {
    let file_path = PathType::General.get_path(path);
    let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
        panic!(
            "Unable to find `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });
    let f = data.parse::<toml::Value>().unwrap_or_else(|e| {
        panic!(
            "Unable to parse `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path.to_string_lossy()));

    let formats = table
        .get("ALLOWED_FORMATS")
        .unwrap_or_else(|| {
            panic!(
                "Unable to find ALLOWED_FORMATS in {}",
                file_path.to_string_lossy()
            )
        })
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "ALLOWED_FORMATS in {} is not an array of strings!",
                file_path.to_string_lossy()
            )
        });

    let mut res = HashSet::default();

    for format in formats {
        let string = format
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "ALLOWED_FORMATS in {} is not an array of strings!",
                    file_path.to_string_lossy()
                )
            })
            .to_owned();
        res.insert(string);
    }

    Ok(res)
}

fn load_allowed_chars(path: &PathBuf) -> Result<HashSet<char>, ConfigError> {
    let file_path = PathType::General.get_path(path);
    let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
        panic!(
            "Unable to find `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });
    let f = data.parse::<toml::Value>().unwrap_or_else(|e| {
        panic!(
            "Unable to parse `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path.to_string_lossy()));

    let raw_string: String = table
        .get("ALLOWED_CHARS")
        .unwrap_or_else(|| {
            panic!(
                "Unable to find ALLOWED_CHARS in {}",
                file_path.to_string_lossy()
            )
        })
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "ALLOWED_CHARS in {} is not a string!",
                file_path.to_string_lossy()
            )
        })
        .to_owned();

    let mut res = HashSet::default();

    raw_string.chars().for_each(|c| {
        res.insert(c);
    });

    Ok(res)
}

fn load_blacklisted_phrases(path: &PathBuf) -> Result<Vec<String>, ConfigError> {
    let file_path = PathType::General.get_path(path);

    let data = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
        panic!(
            "Unable to find `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });
    let f = data.parse::<toml::Value>().unwrap_or_else(|e| {
        panic!(
            "Unable to parse `{}` due to error {}",
            file_path.to_string_lossy(),
            e
        )
    });

    let table = f
        .as_table()
        .unwrap_or_else(|| panic!("Unable to parse {} as table.", file_path.to_string_lossy()));

    let phrases = table
        .get("BLACKLISTED_PHRASES")
        .unwrap_or_else(|| {
            panic!(
                "Unable to find BLACKLISTED_PHRASES in {}",
                file_path.to_string_lossy()
            )
        })
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "BLACKLISTED_PHRASES in {} is not an array of strings!",
                file_path.to_string_lossy()
            )
        });

    let mut res = vec![];

    for phrase in phrases {
        let string = phrase
            .as_str()
            .unwrap_or_else(|| {
                panic!(
                    "BLACKLISTED_PHRASES in {} is not an array of strings!",
                    file_path.to_string_lossy()
                )
            })
            .to_owned();
        res.push(string);
    }

    Ok(res)
}

pub struct Config {
    /// The name of the api which is sent with certain requests.
    api_name: String,

    /// The path to the cache for storing .wav files.
    cache_path: String,

    /// The path where temporary files are stored, and should be deleted from on a crash.
    temp_path: String,

    /// The maximum size of of the cache that may be stored on the system.
    max_cache_size: usize,

    /// The maximum length of a phrase that the api will process.
    word_length_limit: usize,

    /// The maximum speed at which a phrase can be read.
    speed_max_val: f32,

    /// The lowerest speed at which a phrase can be read.
    speed_min_val: f32,

    /// A list of supported speech languages by this api.
    supported_langs: HashMap<String, Language>,

    /// The list of supported file-formats, note that wav is the preferred format due to lower cpu usage.
    allowed_formats: HashSet<String>,

    /// A hashset of chars that the api will accept as input.
    allowed_chars: HashSet<char>,

    /// A list of phrases that are not allowed on this api.
    blacklisted_phrases: Vec<String>,
}

impl Config {
    pub fn new(path: PathBuf) -> Result<Self, ConfigError> {
        Ok(Self {
            api_name: load_env("API_NAME", &path)?,
            cache_path: load_env("CACHE_PATH", &path)?,
            temp_path: load_env("TEMP_PATH", &path)?,
            max_cache_size: load_env("MAX_CACHE_SIZE_MB", &path)?,
            word_length_limit: load_env("CHAR_LENGTH_LIMIT", &path)?,
            speed_max_val: load_env("SPEED_MAX_VAL", &path)?,
            speed_min_val: load_env("SPEED_MIN_VAL", &path)?,
            supported_langs: load_supported_langs(&path)?,
            allowed_formats: load_allowed_formats(&path)?,
            allowed_chars: load_allowed_chars(&path)?,
            blacklisted_phrases: load_blacklisted_phrases(&path)?,
        })
    }
}

//XXX make a getter macro which can automatically generate all these
#[allow(non_snake_case)]
impl Config {
    pub fn API_NAME(&self) -> &str {
        &self.api_name
    }

    pub fn CACHE_PATH(&self) -> &str {
        &self.cache_path
    }

    pub fn TEMP_PATH(&self) -> &str {
        &self.temp_path
    }

    pub fn MAX_CACHE_SIZE(&self) -> usize {
        self.max_cache_size
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
    pub fn fairing() -> AdHoc {
        AdHoc::on_ignite("Custom Configuration Loader", |rocket| {
            Box::pin(async move {
                //Generate Config
                let config = Config::new(PathBuf::from("./config")).unwrap();
                //Save to State
                rocket.manage(config)
            })
        })
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Config {
    type Error = Infallible;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Infallible> {
        let state = req
            .rocket()
            .state::<Config>()
            .expect("Configuration Fairing Not Attached!");
        request::Outcome::Success(state)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use std::path::PathBuf;

    use crate::error::ConfigError;

    use super::load_env;

    #[test]
    #[should_panic]
    fn test_failed_env_load() {
        let _: Result<String, ConfigError> =
            load_env("this_value_does_not_exist123", &PathBuf::from("/"));
    }
}
