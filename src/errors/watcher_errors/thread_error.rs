use std::{sync::{PoisonError, MutexGuard}, process::Child};
use futures::{channel::mpsc::SendError};
use thiserror::Error;
use crate::{errors::script_errors::script_error::ScriptError};

#[derive(Debug, Error)] 
pub enum ThreadError {
    #[error("error with path cache: `{0}`")]
    LockError(String),
    #[error("tokio runtime error: `{0}`")]
    RuntimeError(#[from] std::io::Error),
    #[error("error communicating between threads: `${0}`")]
    RecvSyncError(#[from] tokio::sync::broadcast::error::RecvError),
    #[error("error communicating between threads: `${0}`")]
    SendSyncError(#[from] tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>),
    #[error("error communicating between threads: `${0}`")]
    SendAsyncError(#[from] SendError),
}

pub type LockError<'a, T> = PoisonError<MutexGuard<'a, T>>;

impl <'a,T> From<LockError<'a,T>> for ThreadError {
    fn from(value: LockError<'a,T>) -> Self {
        ThreadError::LockError(value.to_string())
    }
}