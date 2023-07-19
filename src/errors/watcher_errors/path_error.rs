use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("`invalid path: {0}`")]
    InvalidPath(String),
    #[error("`io operation error: {0}`")]
    Io(#[from] std::io::Error),
    #[error("error traversing path")]
    TraversalError,
}
