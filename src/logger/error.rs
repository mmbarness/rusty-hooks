use log::{debug, error, log_enabled, info, Level};
use super::r#struct::Logger;

pub trait ErrorLogging {
    fn log_error_string(message: &String) {
        if log_enabled!(Level::Error) {
            error!("{}", message)
        }
    }
}