use std::path::{Path, PathBuf};

use directories::{BaseDirs};
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
    fn os_specific_log_path() -> Option<PathBuf> {
        let os = std::env::consts::OS;

        let mut home_dir = BaseDirs::new().and_then(|p| Some(p.home_dir().to_path_buf()))?.canonicalize().ok()?;
        let rusty_hooks_log_subdir = Path::new("/rusty-hooks/rusty-hooks.log").to_path_buf();
        let linux_log_path = home_dir.join(&rusty_hooks_log_subdir).to_path_buf();

        let mac_log_path = home_dir.join(Path::new("/Library/Logs").join(&rusty_hooks_log_subdir).to_path_buf());

        home_dir.push(rusty_hooks_log_subdir);

        match os {
            "linux" => Some(home_dir),
            // "macos" => Some(mac_log_path),
            _ => None
        }
    }

    pub fn on_load(level: LevelFilter) -> Result<log4rs::Handle, LoggerError> {
        let file_path = Self::os_specific_log_path();
        let file_path_str = file_path.clone().unwrap();
        let whatever = file_path_str.to_str().unwrap();

        println!("{}", whatever);

        // Build a stderr logger.
        let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
        let stderr_appender = Appender::builder()
            .filter(Box::new(ThresholdFilter::new(level)))
            .build("stderr", Box::new(stderr));
    
        match file_path {
            Some(path) => {
                let window_size = 3; // log0, log1, log2
                let fixed_window_roller = FixedWindowRoller::builder().build("log{}",window_size).unwrap();
        
                let size_limit = 5 * 1024; // 5KB as max log file size to roll
                let size_trigger = SizeTrigger::new(size_limit);
        
                let compound_policy = CompoundPolicy::new(Box::new(size_trigger),Box::new(fixed_window_roller));
                
                let rolling_file = RollingFileAppender::builder()
                    .encoder(Box::new(PatternEncoder::new("{d} {l}::{m}{n}")))
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
                // Only logging to stderr if system isn't mac or linux
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