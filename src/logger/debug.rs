use log::{debug, log_enabled, Level};
use super::structs::Logger;

pub trait DebugLogging {
    fn log_debug_string(message: &String) {
        if log_enabled!(Level::Debug) {
            debug!("{}", message)
        }
    }
}

impl DebugLogging for Logger {}