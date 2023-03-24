use std::{fmt, str::FromStr};
use crate::errors::watcher_errors::spawn_error::SpawnError;

#[derive(Debug)]
pub enum ScriptError {
    ConfigsError,
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    SpawnError(SpawnError),
    GenericMessage(String),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::ConfigsError => 
                write!(f, "error parsing script config file"),
            ScriptError::IoError(e) => 
                write!(f, "error with io operation pertaining to script: {}", e),
            ScriptError::JsonError(e) => 
                write!(f, "error parsing script into or from a json: {}", e),
            ScriptError::SpawnError(e) => 
                write!(f, "error spawning script process: {}", e),
            ScriptError::GenericMessage(e) => 
                write!(f, "error with script: {}", e)
        }
    }
}

impl From<serde_json::Error> for ScriptError {
    fn from(value: serde_json::Error) -> Self {
        ScriptError::JsonError(value)
    }
}

impl From<SpawnError> for ScriptError {
    fn from(value:SpawnError) -> Self {
        ScriptError::SpawnError(value)
    }
}

impl From<std::io::Error> for ScriptError {
    fn from(value:std::io::Error) -> Self {
        ScriptError::IoError(value)
    }
}

impl FromStr for ScriptError {
    fn from_str(s: &str) -> Result<ScriptError, ScriptError> {
        Ok(ScriptError::GenericMessage(s.to_string()))
    }
    type Err = ScriptError;
}