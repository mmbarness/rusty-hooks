use super::{event_error::EventTypeError, thread_error::ThreadError, script_error::ScriptError, timer_error::TimerError, path_error::PathError};


#[derive(Debug)]
pub enum WatcherError<'a> {
    EventTypeError(EventTypeError),
    PathError(PathError),
    ThreadError(ThreadError<'a>),
    TimerError(TimerError<'a>),
    ScriptError(ScriptError)
}