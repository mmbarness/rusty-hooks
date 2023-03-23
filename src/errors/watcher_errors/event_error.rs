use std::{error,fmt};

use strum::ParseError;

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