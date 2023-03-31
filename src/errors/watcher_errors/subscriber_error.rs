use std::path::PathBuf;

use thiserror::Error;
use tokio::sync::{TryLockError, broadcast::error::SendError};
use crate::{errors::runtime_error::enums::RuntimeError, scripts::structs::Script};
use super::{timer_error::TimerError, thread_error::{ThreadError, UnexpectedAnyhowError}, event_error::EventError};

#[derive(Debug, Error)]
pub enum SubscriberError {
    #[error("error while waiting for watched path to stabilize. `{0}`")]
    EventError(#[from] EventError),
    #[error("error accessing paths currently watching: `{0}`")]
    LockError(#[from] TryLockError),
    #[error("error with threads spawned to wait on watched paths `${0}`")]
    RuntimeError(#[from] RuntimeError),
    #[error("error with sending path and scripts to spawn thread: `${0}`")]
    SpawnSendError(#[from] SendError<(PathBuf, Vec<Script>)>),
    #[error("error managing timer: `${0}`")]
    TimerError(#[from] TimerError),
    #[error("`${0}`")]
    ThreadError(#[from] ThreadError),
    #[error("error removing path from watched paths: `{0}`")]
    UnsubscribeSendError(#[from] SendError<PathBuf>),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl UnexpectedAnyhowError for SubscriberError {}