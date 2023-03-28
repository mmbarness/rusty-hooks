use std::{error,fmt};
use strum::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventError {
    NotifyError(#[from] notify::Error),
    TypeError(#[from] EventTypeError),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::NotifyError(e) => 
                write!(f, "error while watching file system events: {}", e.to_string()),
            EventError::TypeError(e) => 
                write!(f, "error parsing event type as string: {}", e.to_string()),
        }
    }
}

#[derive(Debug)]
pub enum EventTypeError {
    ParseString(ParseError)
}

impl error::Error for EventTypeError {}

impl fmt::Display for EventTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventTypeError::ParseString(e) => 
                write!(f, "error parsing event type as string: {}", e.to_string()),
        }
    }
}