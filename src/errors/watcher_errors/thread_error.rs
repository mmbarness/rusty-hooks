use std::{sync::{PoisonError, MutexGuard}, process::Child, fmt};
use futures::{channel::mpsc::SendError};
use thiserror::Error;
use crate::{errors::script_errors::script_error::ScriptError};

#[derive(Debug, Error)] 
pub enum ThreadError {
    LockError(String),
    RuntimeError(#[from] std::io::Error),
    RecvSyncError(#[from] tokio::sync::broadcast::error::RecvError),
    SendSyncError(#[from] tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>),
    SendAsyncError(#[from] SendError),
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

impl <'a,T> From<LockError<'a,T>> for ThreadError {
    fn from(value: LockError<'a,T>) -> Self {
        ThreadError::LockError(value.to_string())
    }
}