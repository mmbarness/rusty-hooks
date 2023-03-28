use std::fmt;
use strum::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpawnError {
    IoError(#[from] std::io::Error),
    ParseError(#[from] ParseError),
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
