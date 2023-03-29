use thiserror::Error;
use tokio::{sync::TryLockError, task::JoinError};

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("error executing spawned task: `{0}`")]
    JoinError(#[from] JoinError),
    #[error("error locking onto tokio runtime: `{0}`")]
    LockError(#[from] TryLockError)
}