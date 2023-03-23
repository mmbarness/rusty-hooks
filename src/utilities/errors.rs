use core::fmt;
use std::{time::Duration, sync::PoisonError};
use tokio::sync::MutexGuard;

#[derive(Debug)]
pub enum TimerError<'a> {
    AsyncLockerror(PoisonError<MutexGuard<'a, (Duration,tokio::time::Instant)>>)
}

pub type TimerLockError<'a> = PoisonError<MutexGuard<'a, (Duration,tokio::time::Instant)>>;

impl <'a> fmt::Display for TimerError<'a>{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimerError::AsyncLockerror(e) => 
                write!(f, "error accessing timer's shared state: {}", e)
        }
    }
}

impl <'a> From<TimerLockError<'a>> for TimerError<'a> {
    fn from(value: TimerLockError<'a>) -> Self {
        TimerError::AsyncLockerror(value)
    }
}
