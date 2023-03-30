use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}, path::{Path, PathBuf}};
use tokio::runtime::Runtime;
use crate::utilities::{thread_types::Channel,timer::Timer};
use crate::logger::{debug::DebugLogging, structs::Logger, info::InfoLogging, error::ErrorLogging};
use crate::errors::watcher_errors::thread_error::ThreadError;

pub trait Utilities {
    fn hasher(path: &String) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(& mut hasher);
        hasher.finish()
    }

    fn format_unvalidated_path(segments: &Vec<&String>) -> String {
        let init_path = "".to_string();
        segments.iter().fold(init_path, |path:String, segment| {
            format!("{}{}", path, segment)
        })
    }

    fn build_path(segments: &Vec<&String>) -> Option<PathBuf> {
        let init_path = PathBuf::new();
        let path = segments.iter().fold(init_path, |path:PathBuf, segment| {
            path.to_path_buf().join(segment)
        });
        Self::log_path(&path, log::Level::Debug);
        Self::verify_path(path)
    }

    fn verify_path(path: PathBuf) -> Option<PathBuf> {
        path.exists().then_some(path)
    }

    fn log_path(path: &Path, log_level: log::Level) -> () {
        let path_string = path.to_str().unwrap_or("error parsing path into string");
        let message = &format!("path: {}", path_string);
        match log_level {
            log::Level::Debug => {
                Logger::log_debug_string(message);
            },
            log::Level::Info => {
                Logger::log_info_string(message)
            },
            log::Level::Error => {
                Logger::log_error_string(message)
            },
            _ => {
                Logger::log_debug_string(message);
            }
        };
    }

    fn new_channel<T:std::clone::Clone>() -> Channel<T> {
        tokio::sync::broadcast::channel::<T>(16)
    }

    fn new_runtime(num_threads: usize, thread_name: &String) -> Result<Runtime, ThreadError> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_threads)
            .thread_name(thread_name)
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()?)
    }

    fn new_timer(wait_duration: i64) -> Timer {
        Timer::new(wait_duration)
    }
}