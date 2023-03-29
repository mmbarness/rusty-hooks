use thiserror::Error;
use tokio::sync::TryLockError;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("error locking onto timer controller: `{0}`")]
    LockError(#[from] TryLockError)
}