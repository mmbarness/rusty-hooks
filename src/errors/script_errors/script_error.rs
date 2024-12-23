use crate::errors::watcher_errors::spawn_error::SpawnError;
use anyhow::anyhow;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("error while loading configs: `{0}`")]
    ConfigError(#[from] ScriptConfigError),
    #[error("io error while reading in user scripts: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error spawning script process: `{0}`")]
    SpawnError(#[from] SpawnError),
}

#[derive(Debug, Error)]
pub enum ScriptConfigError {
    #[error("io error while reading in user scripts: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error parsing user script_config.json: `{0}`")]
    JsonError(#[from] serde_json::Error),
    #[error("error parsing scripts.yml: `{0}`")]
    YAMLError(#[from] serde_yaml::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<String> for ScriptConfigError {
    fn from(value: String) -> Self {
        ScriptConfigError::UnexpectedError(anyhow!(value))
    }
}
