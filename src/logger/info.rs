use log::{log_enabled, info, Level};

pub trait InfoLogging {
    fn log_info_string(message: &str) {
        if log_enabled!(Level::Info) {
            info!("{}", message);
        }
    }
}
