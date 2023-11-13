use std::path::{Path, PathBuf};
use directories::BaseDirs;
use log::LevelFilter;
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
use super::structs::{Logger, LoggerError};

impl Logger {
    fn log_path_if_linux_or_mac() -> Option<PathBuf> {
        let os = std::env::consts::OS;

        match os {
            "linux" => Self::linux_log_path(),
            "mac" => Self::mac_log_path(),
            _ => None
        }
    }

    fn linux_log_path() -> Option<PathBuf> {
        let log_path = Path::new("/var/log/rusty-hooks.log").to_path_buf();
        Some(log_path)
    }

    fn mac_log_path() -> Option<PathBuf> {
        let home_dir = BaseDirs::new().and_then(|p| Some(p.home_dir().to_path_buf()))?.canonicalize().ok()?;
        let rusty_hooks_log_subdir = Path::new("Library/rusty-hooks/logs/rusty-hooks.log").to_path_buf();
        Some([
            home_dir.as_path(),
            rusty_hooks_log_subdir.as_path()
        ].iter().collect())
    }

    pub fn on_load(level: LevelFilter) -> Result<log4rs::Handle, LoggerError> {
        let file_path = Self::log_path_if_linux_or_mac();
        let encoder= Box::new(PatternEncoder::new("ts={d} level={l} message=\"{m}\" src={f} pid={P} {n}"));
        // Build a stderr logger.
        let stderr = ConsoleAppender::builder()
            .encoder(encoder.clone())
            .target(Target::Stderr)
            .build();

        let stderr_appender = Appender::builder()
            .filter(Box::new(ThresholdFilter::new(level)))
            .build("stderr", Box::new(stderr));
        match file_path {
            Some(path) => {
                let window_size = 3; // log0, log1, log2
                let fixed_window_roller = FixedWindowRoller::builder().build("log{}",window_size).unwrap();

                let size_limit = 5000 * 1024; // 5MB as max log file size to roll
                let size_trigger = SizeTrigger::new(size_limit);

                let compound_policy = CompoundPolicy::new(Box::new(size_trigger),Box::new(fixed_window_roller));

                let rolling_file = RollingFileAppender::builder()
                    .encoder(encoder)
                    .build(path, Box::new(compound_policy))?;

                // Log Trace level output to file where trace is the default level
                // and the programmatically specified level to stderr.
                let config = Config::builder()
                    .appender(Appender::builder().build("logfile", Box::new(rolling_file)))
                    .appender(stderr_appender)
                    .build(
                        Root::builder()
                            .appender("logfile")
                            .appender("stderr")
                            .build(LevelFilter::Trace),
                    )
                    .unwrap();

                Ok(log4rs::init_config(config)?)
            },
            None => {
                // Only logging to stderr if system isn't linux
                let config = Config::builder()
                    .appender(stderr_appender)
                    .build(
                        Root::builder()
                            .appender("stderr")
                            .build(LevelFilter::Trace),
                    )
                    .unwrap();
                Ok(log4rs::init_config(config)?)
            }
        }

    }
}
