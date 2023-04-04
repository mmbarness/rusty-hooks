use log::{log_enabled, info, Level};

pub trait InfoLogging {
    fn log_info_string(message: &String) {
        if log_enabled!(Level::Info) {
            info!("{}", message);
        }
    }
}
