use log::{error, log_enabled, Level};

pub trait ErrorLogging {
    fn log_error_string(message: &str) {
        if log_enabled!(Level::Error) {
            error!("{}", message)
        }
    }
}