use log::{debug, error, log_enabled, info, Level};

pub struct Logger {}

impl Logger {
    pub fn on_load () -> () {
        env_logger::init();
    }
}

pub trait InfoLogging {
    fn log_info_string(message: &String) {
        if log_enabled!(Level::Info) {
            info!("{}", message);
        }
    }
}

pub trait DebugLogging {
    fn log_debug_string(message: &String) {
        if log_enabled!(Level::Debug) {
            debug!("{}", message)
        }
    }
}

pub trait ErrorLogging {
    fn log_error_string(message: &String) {
        if log_enabled!(Level::Error) {
            error!("{}", message)
        }
    }
}

impl InfoLogging for Logger {}
impl DebugLogging for Logger {}
impl ErrorLogging for Logger {}