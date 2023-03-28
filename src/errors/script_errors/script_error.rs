use std::fmt;
use thiserror::Error;

use crate::{errors::watcher_errors::spawn_error::SpawnError};

#[derive(Debug, Error)]
pub enum ScriptError {
    ConfigError(ConfigError),
    IoError(std::io::Error),
    SpawnError(SpawnError),
    GenericMessage {
        #[from]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(e) => 
                write!(f, "io error while reading in user scripts: {}", e),
            ConfigError::JsonError(e) => 
                write!(f, "error parsing user script_config.json: {}", e)
        }
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::ConfigError(e) => 
                write!(f, "error while loading configs: {}", e),
            ScriptError::IoError(e) => 
                write!(f, "error with io operation pertaining to script: {}", e),
            ScriptError::SpawnError(e) => 
                write!(f, "error spawning script process: {}", e),
            ScriptError::GenericMessage { source } => {
                write!(f, "error with script: {}", source.to_string())
            }
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        ConfigError::JsonError(value)
    }
}

impl From<ConfigError> for ScriptError {
    fn from(value: ConfigError) -> Self {
        ScriptError::ConfigError(value)
    }
}

impl From<SpawnError> for ScriptError {
    fn from(value:SpawnError) -> Self {
        ScriptError::SpawnError(value)
    }
}