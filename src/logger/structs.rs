use anyhow::anyhow;
use log::SetLoggerError;
use thiserror::Error;
pub struct Logger {}

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("error setting logger level: `{0}`")]
    SetLoggerError(#[from] SetLoggerError),
    #[error("error while configuring log file: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<String> for LoggerError {
    fn from(value: String) -> Self {
        LoggerError::UnexpectedError(anyhow!(value))
    }
}
