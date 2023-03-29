use thiserror::Error;
use tokio::sync::TryLockError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("error locking onto tokio runtime: `{0}`")]
    LockError(#[from] TryLockError)
}