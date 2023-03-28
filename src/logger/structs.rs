use log::{log_enabled, Level, info, LevelFilter};
use super::error::ErrorLogging;

pub struct Logger {}

impl Logger {
    pub fn on_load (level: LevelFilter) -> () {
        env_logger::builder().filter_level(level).init();
        if log_enabled!(Level::Info) {
            info!("{}", "log level set to info");
        }
        if log_enabled!(Level::Debug) {
            info!("{}", "log level set to debug");
        }
        if log_enabled!(Level::Error) {
            info!("{}", "log level set to error");
        }
    }
}

impl ErrorLogging for Logger {}