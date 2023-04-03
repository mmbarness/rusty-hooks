use thiserror::Error;
use crate::errors::watcher_errors::spawn_error::SpawnError;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("error while loading configs: `{0}`")]
    ConfigError(#[from] ScriptConfigError),
    #[error("io error while reading in user scripts: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error spawning script process: `{0}`")]
    SpawnError(#[from] SpawnError),
    #[error("error with script: `{0}`")]
    GenericMessage(String)
}

#[derive(Debug, Error)]
pub enum ScriptConfigError {
    #[error("io error while reading in user scripts: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error parsing user script_config.json: `{0}`")]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}