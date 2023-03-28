use std::fmt;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum PathError {
    TraversalError,
    Io(#[from] std::io::Error),
    UnsubscribeError(String)
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::UnsubscribeError(e) => 
                write!(f, "{}", e.to_string()),
            PathError::Io(e) => 
                write!(f, "io operation error: {}", e.to_string()),
            PathError::TraversalError => 
                write!(f, "error traversing file structure"),
        }
    }
}
