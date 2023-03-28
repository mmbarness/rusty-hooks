use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("`io operation error: {0}`")]
    Io(#[from] std::io::Error),
    #[error("error traversing path")]
    TraversalError,
    #[error("`{0}`")]
    UnsubscribeError(String)
}
