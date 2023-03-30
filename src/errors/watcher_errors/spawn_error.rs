use strum::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("io operation error: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error parsing script while attempting to spawn new process: `{0}`")]
    ParseError(#[from] ParseError),
    #[error("ierror with path arguments provided to command spawner: `{0}`")]
    ArgError(String),
}