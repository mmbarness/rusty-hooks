use log::{debug, log_enabled, Level};

pub trait DebugLogging {
    fn log_debug_string(message: &String) {
        if log_enabled!(Level::Debug) {
            debug!("{}", message)
        }
    }
}
