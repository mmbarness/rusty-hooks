use log::{debug, error, log_enabled, info, Level};

use super::r#struct::Logger;

pub trait InfoLogging {
    fn log_info_string(message: &String) {
        if log_enabled!(Level::Info) {
            info!("{}", message);
        }
    }
}

impl InfoLogging for Logger {}
