use std::fmt;
use strum::ParseError;


#[derive(Debug)]
pub enum SpawnError {
    IoError(std::io::Error),
    ParseError(ParseError),
    ArgError(String),
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnError::IoError(e) => 
                write!(f, "error with io operation while spawning script: {}", e),
            SpawnError::ParseError(e) => 
                write!(f, "error parsing script while attempting to spawn new process: {}", e),
            SpawnError::ArgError(e) =>  
                write!(f, "error with path arguments provided to command spawner: {}", e)
        }
    }
}

impl From<std::io::Error> for SpawnError {
    fn from(value:std::io::Error) -> Self {
        SpawnError::IoError(value)
    }
}

impl From<ParseError> for SpawnError {
    fn from(value: ParseError) -> Self {
        SpawnError::ParseError(value)
    }
}
