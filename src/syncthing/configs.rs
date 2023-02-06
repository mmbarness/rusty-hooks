#![deny(unconditional_recursion)]
use std::{str::FromStr, num::ParseIntError};
use dotenv::dotenv;
use strum::ParseError;
use thiserror::Error;
use strum_macros::{EnumString, AsRefStr};
use super::logger::{Logger, InfoLogging, DebugLogging};

#[derive(Debug, Clone)]
pub struct Configs {
    pub auth_key: AuthKey,
    pub port: Port,
    pub address: Address,
    pub request_interval: RequestInterval,
    pub script_delay: ScriptDelay,
    pub scripts_path: ScriptsPath,
}

#[derive(Debug, Error)]
pub enum ConfigError{
    #[error("Unable to find env var: `{0}`")]
    MissingError(String),
    #[error("Error while parsing env vars: `{0}`")]
    ParseError(String),
    #[error("Error with strum: `{0}`")]
    StrumParseError(#[from] ParseError),
    #[error("Error with strum: `{0}`")]
    ParseIntError(#[from] ParseIntError),
}

#[derive(AsRefStr, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum ConfigValues {
    AuthKey(AuthKey),
    Port(Port),
    Address(Address),
    RequestInterval(RequestInterval),
    ScriptDelay(ScriptDelay),
    ScriptPath(ScriptsPath),
}

pub type AuthKey = String;
pub type Port = u16;
pub type Address = String;
pub type RequestInterval = u64;
pub type ScriptDelay = u8;
pub type ScriptsPath = String;

pub trait ConfigValue {}

impl Configs {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_env_vars();

        let auth_key = match Self::get_var( "AUTH_KEY".to_string())? {
            Some(ConfigValues::AuthKey(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing auth_key from .env file, please check it and try again".into())),
            None => return Err(ConfigError::MissingError("unable to find auth_key in .env".to_string()))
        };

        let port = match Self::get_var( "PORT".to_string())? {
            Some(ConfigValues::Port(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing port from .env file, please check it and try again".to_string())),
            None => {
                Logger::log_info_string(&"didn\'t find port in .env, using default of 8384".to_string());
                8384
            }
        };

        let address = match Self::get_var( "ADDRESS".to_string())? {
            Some(ConfigValues::Address(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing address from .env file, please check it and try again".to_string())),
            None => {
                Logger::log_info_string(&"didn\'t find address in .env, assuming localhost".to_string());
                "http://127.0.0.1".to_string()
            }
        };

        let request_interval = match Self::get_var( "REQUEST_INTERVAL".to_string())? {
            Some(ConfigValues::RequestInterval(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing request interval from .env file, please check it and try again".to_string())),
            None => {
                Logger::log_info_string(&"didn\'t find a request_interval in .env, using default of every 60 seconds".to_string());
                60
            }
        };

        let script_delay = match Self::get_var( "SCRIPT_DELAY".to_string())? {
            Some(ConfigValues::ScriptDelay(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing request interval from .env file, please check it and try again".to_string())),
            None => {
                Logger::log_info_string(&"didn\'t find a script_delay time in .env, using default of 1 minute".to_string());
                1
            }
        };

        let scripts_path = match Self::get_var( "SCRIPTS_PATH".to_string())? {
            Some(ConfigValues::ScriptPath(c)) => c,
            Some(_) => return Err(ConfigError::ParseError("error parsing request interval from .env file, please check it and try again".to_string())),
            None => {
                Logger::log_info_string(&"didn\'t find a configured scripts path in .env, using default of ../scripts".to_string());
                "../scripts".to_string()
            }
        };

        Ok(Configs {
            address,
            auth_key,
            port,
            request_interval,
            script_delay,
            scripts_path,
        })
    }

    fn load_env_vars() -> () {
        match dotenv() {
            Ok(_) => {
                Logger::log_info_string(&".env file found, using...".to_string())
            }
            Err(_) => {
                Logger::log_info_string(&".env file not found, looking elsewhere".to_string())
            }
        };
    }

    fn get_var(config_value: String) -> Result<Option<ConfigValues>, ConfigError> {
        Logger::log_debug_string(&format!("looking for: {}", &config_value));

        let valid_var = match std::env::var(&config_value) {
            Ok(url) => url,
            Err(_) => return Ok(None)
        };

        let config_value_variant = match ConfigValues::from_str(&config_value) {
            Ok(val) => val,
            Err(e) => return Err(e.into()),
        };

        Logger::log_debug_string(&format!("found value for: {}", &config_value_variant.as_ref()));

        match config_value_variant {
            ConfigValues::AuthKey(_) => Ok(Some(ConfigValues::AuthKey(valid_var))),
            ConfigValues::Address(_) => Ok(Some(ConfigValues::Address(valid_var))),
            ConfigValues::Port(_) => Ok(Some(ConfigValues::Port(valid_var.parse::<u16>()?))),
            ConfigValues::RequestInterval(_) => Ok(Some(ConfigValues::RequestInterval(valid_var.parse::<u64>()?))),
            ConfigValues::ScriptDelay(_) => Ok(Some(ConfigValues::ScriptDelay(valid_var.parse::<u8>()?))),
            ConfigValues::ScriptPath(_) => Ok(Some(ConfigValues::ScriptPath(valid_var))),
        }

    }
}