use std::{error, sync::{PoisonError, MutexGuard}, process::Child, fmt, collections::HashMap, path::PathBuf};
use futures::{channel::mpsc::SendError};
use crate::watcher::watcher_scripts::{WatcherScripts, Script};

use super::script_error::ScriptError;
impl error::Error for ThreadError<'_> {}

#[derive(Debug)] 
pub enum ThreadError<'a> {
    PathsLockError(PoisonError<MutexGuard<'a, HashMap<u64, (PathBuf, Vec<Script>)>>>),
    RuntimeLockError(String),
    RecvSyncError(tokio::sync::broadcast::error::RecvError),
    SendSyncError(tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>),
    SendAsyncError(SendError),
}

pub type PathsLockError<'a> = PoisonError<MutexGuard<'a, HashMap<u64, (PathBuf, Vec<Script>)>>>;

impl fmt::Display for ThreadError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadError::PathsLockError(e) => 
                write!(f, "error while accessing paths queue: {}", e),
            ThreadError::RuntimeLockError(e) => 
                write!(f, "error while accessing paths queue: {}", e),
            ThreadError::RecvSyncError(e) => 
                write!(f, "error communicating between threads: {}", e),
            ThreadError::SendSyncError(e) => 
                write!(f, "error communicating between threads: {}", e),
            ThreadError::SendAsyncError(e) => 
                write!(f, "error communicating async between threads: {}", e)
        }
    }
}

impl <'a> From<PathsLockError<'a>> for ThreadError<'a> {
    fn from(value: PathsLockError<'a>) -> Self {
        ThreadError::PathsLockError(value)
    }
}

impl From<SendError> for ThreadError<'_> {
    fn from(value: SendError) -> Self {
        ThreadError::SendAsyncError(value)
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for ThreadError<'_> {
    fn from(value: tokio::sync::broadcast::error::RecvError) -> Self {
        ThreadError::RecvSyncError(value)
    }
}

impl From<tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>> for ThreadError<'_> {
    fn from(value: tokio::sync::broadcast::error::SendError<Result<Child, ScriptError>>) -> Self {
        ThreadError::SendSyncError(value)
    }
}