use log::{LevelFilter, SetLoggerError};
use log4rs::{
    append::{
        console::{
            ConsoleAppender, 
            Target
        },
        rolling_file::{
            RollingFileAppender, 
            policy::compound::{
                CompoundPolicy, 
                roll::fixed_window::FixedWindowRoller, 
                trigger::size::SizeTrigger
            }
        },
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use thiserror::Error;
use super::{error::ErrorLogging, debug::DebugLogging, info::InfoLogging};

pub struct Logger {}

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("error setting logger level: `{0}`")]
    SetLoggerError(#[from] SetLoggerError),
    #[error("error while configuring log file: `{0}`")]
    IoError(#[from] std::io::Error)
}

impl Logger {
    pub fn on_load(level: LevelFilter) -> Result<log4rs::Handle, LoggerError> {
        let file_path = "/tmp/rusty-hooks/rusty-hooks.log";
    
        // Build a stderr logger.
        let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

        let window_size = 3; // log0, log1, log2
        let fixed_window_roller = FixedWindowRoller::builder().build("log{}",window_size).unwrap();

        let size_limit = 5 * 1024; // 5KB as max log file size to roll
        let size_trigger = SizeTrigger::new(size_limit);

        let compound_policy = CompoundPolicy::new(Box::new(size_trigger),Box::new(fixed_window_roller));
        
        let rolling_file = RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} {l}::{m}{n}")))
            .build(file_path, Box::new(compound_policy))?;
    
        // Log Trace level output to file where trace is the default level
        // and the programmatically specified level to stderr.
        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(rolling_file)))
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level)))
                    .build("stderr", Box::new(stderr)),
            )
            .build(
                Root::builder()
                    .appender("logfile")
                    .appender("stderr")
                    .build(LevelFilter::Trace),
            )
            .unwrap();

        Ok(log4rs::init_config(config)?)
    }
}

impl<T: Logging> DebugLogging for T {}
impl <T: Logging> ErrorLogging for T {}
impl <T: Logging> InfoLogging for T {}
impl Logging for Logger {}

pub trait Logging {}
