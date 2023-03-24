use std::fmt;


#[derive(Debug)]
pub enum PathError {
    TraversalError,
    Io(std::io::Error),
    UnsubscribeError(String)
}

impl From<std::io::Error> for PathError {
    fn from(value: std::io::Error) -> Self {
        PathError::Io(value)
    }
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
