use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}};
use tokio::runtime::Runtime;
use crate::{utilities::thread_types::Channel, errors::watcher_errors::thread_error::ThreadError};
use crate::utilities::timer::Timer;

pub trait Utilities {
    fn hasher(path: &String) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(& mut hasher);
        hasher.finish()
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