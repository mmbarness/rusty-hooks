use log::{log_enabled, Level, info, LevelFilter};
use super::{error::ErrorLogging, debug::DebugLogging, info::InfoLogging};

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

impl<T: Logging> DebugLogging for T {}
impl <T: Logging> ErrorLogging for T {}
impl <T: Logging> InfoLogging for T {}
impl Logging for Logger {}

pub trait Logging {}
