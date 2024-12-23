use strum::ParseError;
use thiserror::Error;
use tokio::sync::broadcast::error::RecvError;

use super::subscriber_error::SubscriptionError;
use crate::errors::shared_errors::thread_errors::ThreadError;

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("io operation error: `{0}`")]
    IoError(#[from] std::io::Error),
    #[error("error parsing script while attempting to spawn new process: `{0}`")]
    ParseError(#[from] ParseError),
    #[error("ierror with path arguments provided to command spawner: `{0}`")]
    ArgError(String),
    #[error("error with sending path and scripts to spawn thread: `${0}`")]
    RecvError(RecvError),
    #[error("`{0}`")]
    ThreadError(#[from] ThreadError),
    #[error("`{0}`")]
    ScriptError(String),
    #[error("error unsubscribing: `{0}`")]
    UnsubscriberError(#[from] SubscriptionError),
}
