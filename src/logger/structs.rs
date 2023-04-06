use anyhow::anyhow;
use log::SetLoggerError;
use thiserror::Error;
use super::{error::ErrorLogging, debug::DebugLogging, info::InfoLogging};

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

impl<T: Logging> DebugLogging for T {}
impl <T: Logging> ErrorLogging for T {}
impl <T: Logging> InfoLogging for T {}
impl Logging for Logger {}

pub trait Logging {}

impl From<String> for LoggerError {
    fn from(value: String) -> Self {
        LoggerError::UnexpectedError(anyhow!(value))
    }
}