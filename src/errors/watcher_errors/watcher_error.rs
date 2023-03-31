use thiserror::Error;
use crate::errors::{script_errors::script_error::ScriptError, runtime_error::enums::RuntimeError};
use super::{
    event_error::EventError,
    thread_error::ThreadError,
    path_error::PathError, subscriber_error::SubscriberError
};

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("error with file system events: `{0}`")]
    EventError(#[from] EventError),
    #[error("error handling watched paths: `{0}`")]
    PathError(#[from] PathError),
    #[error("error managing subscription state: `${0}`")]
    SubscriberError(#[from] SubscriberError),
    #[error("error with watcher threadpool: `{0}`")]
    RuntimeError(#[from] RuntimeError),
    #[error("error communicating between threads: `${0}`")]
    ThreadError(#[from] ThreadError),
    #[error("error handling user scripts: `${0}`")]
    ScriptError(#[from] ScriptError)
}