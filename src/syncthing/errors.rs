use std::{error, fmt};
use std::str::FromStr;
use strum::ParseError;

use super::configs::ConfigError;

#[derive(Debug)]
pub enum EventTypesError {
    ParseString(ParseError)
}

impl error::Error for EventTypesError {}

impl fmt::Display for EventTypesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventTypesError::ParseString(e) => 
                write!(f, "error parsing event types as string: {}", e.to_string()),
        }
    }
}

impl fmt::Display for SyncthingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncthingError::GenericMessage(message) => 
                write!(f, "{}", message),
            SyncthingError::ConfigError(e) => 
                write!(f, "configuration error: {}", e.to_string()),
            SyncthingError::ApiError => 
                write!(f, "{}", "error talking to syncthing: "),
            SyncthingError::NoNewEvents => 
                write!(f, "{}", "no new events to look at: "),
            SyncthingError::ParseError(e) => 
                write!(f, "error parsing: {}", e.to_string()),
            SyncthingError::EventTypeError(e) => 
                write!(f, "error w/ event types: {}", e.to_string()),
            SyncthingError::ReqwestError(e) => 
                write!(f, "error making http request via reqwest: {}", e.to_string()),
            SyncthingError::SerdeError(e) => 
                write!(f, "error converting resp json into structs: {}", e.to_string()),
            SyncthingError::SpawnError(e) => 
                write!(f, "error spawning process: {}", e.to_string()),
        }
    }
}

impl From<ConfigError> for SyncthingError {
    fn from(value: ConfigError) -> Self {
        SyncthingError::ConfigError(value)
    }
}

impl From<EventTypesError> for SyncthingError {
    fn from(value: EventTypesError) -> Self {
        SyncthingError::EventTypeError(value)
    }
}

impl From<serde_json::Error> for SyncthingError {
    fn from(value: serde_json::Error) -> Self {
        SyncthingError::SerdeError(value)
    }
}

impl From<reqwest::Error> for SyncthingError {
    fn from(value: reqwest::Error) -> Self {
        SyncthingError::ReqwestError(value)
    }
}

impl From<std::io::Error> for SyncthingError {
    fn from(value:std::io::Error) -> Self {
        SyncthingError::SpawnError(value)
    }
}

impl error::Error for SyncthingError {}

impl FromStr for SyncthingError {
    fn from_str(s: &str) -> Result<SyncthingError, SyncthingError> {
        Ok(SyncthingError::GenericMessage(s.to_string()))
    }
    type Err = SyncthingError;
}

#[derive(Debug)]
pub enum SyncthingError {
    ApiError,
    ConfigError(ConfigError),
    EventTypeError(EventTypesError),
    GenericMessage(String),
    NoNewEvents,
    ParseError(ParseError),
    ReqwestError(reqwest::Error),
    SerdeError(serde_json::Error),
    SpawnError(std::io::Error)
}
