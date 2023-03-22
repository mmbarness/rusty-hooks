
#[derive(Debug)]
pub enum PathError {
    TraversalError,
    Io(std::io::Error),
}

impl From<std::io::Error> for PathError {
    fn from(value: std::io::Error) -> Self {
        PathError::Io(value)
    }
}