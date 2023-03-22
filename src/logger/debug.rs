use std::fmt::Debug;

use log::{debug, error, log_enabled, info, Level};
use super::r#struct::Logger;

pub trait DebugLogging {
    fn log_debug_string(message: &String) {
        if log_enabled!(Level::Error) {
            debug!("{}", message)
        }
    }
}

impl DebugLogging for Logger {}