use std::{error, sync::{PoisonError, MutexGuard}, process::Child, fmt};
use futures::{channel::mpsc::SendError};
use crate::{errors::script_errors::script_error::ScriptError};
impl error::Error for ThreadError {}

#[derive(Debug)] 
pub enum ThreadError {
    LockError(String),
    RuntimeError(std::io::Error),
    RecvSyncError(tokio::sync::broadcast::error::RecvError),
    SendSyncError(tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>),
    SendAsyncError(SendError),
}

pub type LockError<'a, T> = PoisonError<MutexGuard<'a, T>>;

impl fmt::Display for ThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ThreadError::LockError(e) => 
                write!(f, "error with path cache: {}", e),
            ThreadError::RuntimeError(e) => 
                write!(f, "tokio runtime error: {}", e),
            ThreadError::RecvSyncError(e) => 
                write!(f, "error communicating between threads: {}", e),
            ThreadError::SendSyncError(e) => 
                write!(f, "error communicating between threads: {}", e),
            ThreadError::SendAsyncError(e) => 
                write!(f, "error communicating async between threads: {}", e)
        }
    }
}

impl From<std::io::Error> for ThreadError {
    fn from(value: std::io::Error) -> Self {
        ThreadError::RuntimeError(value)
    }
}

impl <'a,T> From<LockError<'a,T>> for ThreadError {
    fn from(value: LockError<'a,T>) -> Self {
        ThreadError::LockError(value.to_string())
    }
}

impl From<SendError> for ThreadError {
    fn from(value: SendError) -> Self {
        ThreadError::SendAsyncError(value)
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for ThreadError {
    fn from(value: tokio::sync::broadcast::error::RecvError) -> Self {
        ThreadError::RecvSyncError(value)
    }
}

impl From<tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>> for ThreadError {
    fn from(value: tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>) -> Self {
        ThreadError::SendSyncError(value)
    }
}