use thiserror::Error;
use crate::errors::{runtime_error::enums::RuntimeError, script_errors::script_error::ScriptError, shared_errors::thread_errors::ThreadError};
use super::{
    event_error::EventError,
    path_error::PathError, subscriber_error::SubscriptionError
};

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("error with file system events: `{0}`")]
    EventError(#[from] EventError),
    #[error("error handling watched paths: `{0}`")]
    PathError(#[from] PathError),
    #[error("error managing subscription state: `${0}`")]
    SubscriberError(#[from] SubscriptionError),
    #[error("error with watcher threadpool: `{0}`")]
    RuntimeError(#[from] RuntimeError),
    #[error("error communicating between threads: `${0}`")]
    ThreadError(#[from] ThreadError),
    #[error("error handling user scripts: `${0}`")]
    ScriptError(#[from] ScriptError)
}
