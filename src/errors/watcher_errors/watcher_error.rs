use std::fmt;
use super::{event_error::{EventTypeError, EventError}, thread_error::ThreadError, script_error::ScriptError, path_error::PathError};

impl std::error::Error for WatcherError {}

#[derive(Debug)]
pub enum WatcherError {
    EventError(EventError),
    PathError(PathError),
    ThreadError(ThreadError),
    ScriptError(ScriptError)
}

impl fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WatcherError::EventError(e) => 
                write!(f, "error with file system events: {}", e),
            WatcherError::PathError(e) => 
                write!(f, "error handling watched paths: {}", e),
            WatcherError::ThreadError(e) => 
                write!(f, "error communicating between threads: {}", e),
            WatcherError::ScriptError(e) => 
                write!(f, "error handling user scripts: {}", e),
        }
    }
}

impl From<ThreadError> for WatcherError {
    fn from(value: ThreadError) -> Self {
        WatcherError::ThreadError(value)
    }
}

impl From<EventError> for WatcherError {
    fn from(value: EventError) -> Self {
        WatcherError::EventError(value)
    }
}