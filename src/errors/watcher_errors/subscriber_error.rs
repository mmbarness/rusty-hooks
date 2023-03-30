use thiserror::Error;
use tokio::sync::TryLockError;
use crate::errors::runtime_error::enums::RuntimeError;
use super::{timer_error::TimerError, thread_error::ThreadError, event_error::EventError};

#[derive(Debug, Error)]
pub enum SubscriberError {
    #[error("error while waiting for watched path to stabilize. `{0}`")]
    EventError(#[from] EventError),
    #[error("error accessing paths currently watching: `{0}`")]
    LockError(#[from] TryLockError),
    #[error("error with threads spawned to wait on watched paths `${0}`")]
    RuntimeError(#[from] RuntimeError),
    #[error("error managing timer: `${0}`")]
    TimerError(#[from] TimerError),
    #[error("`${0}`")]
    ThreadError(#[from] ThreadError)
}