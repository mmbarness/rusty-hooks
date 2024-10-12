use std::sync::{PoisonError, MutexGuard};
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Debug, Error)]
pub enum ThreadError {
    #[error("error joining on a task: `{0}`")]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("error with path cache: `{0}`")]
    LockError(String),
    #[error("tokio runtime error: `{0}`")]
    RuntimeError(#[from] std::io::Error),
    #[error("error communicating between threads: `${0}`")]
    RecvError(#[from] tokio::sync::broadcast::error::RecvError),
}

pub type LockError<'a, T> = PoisonError<MutexGuard<'a, T>>;

impl <'a,T> From<LockError<'a,T>> for ThreadError {
    fn from(value: LockError<'a,T>) -> Self {
        ThreadError::LockError(value.to_string())
    }
}

pub trait UnexpectedAnyhowError {
    fn new_unexpected_error<T: std::convert::From<anyhow::Error>> (message: String) -> T {
        let generic_anyhow_error = anyhow::format_err!(message);
        let to_return:T = generic_anyhow_error.into();
        to_return
    }
}

impl UnexpectedAnyhowError for ThreadError {}
