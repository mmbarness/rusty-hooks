use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}, sync::{Arc}};
use tokio::{runtime::Runtime};
use crate::watcher::types::Channel;
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

    fn new_runtime(num_threads: usize, thread_name: &String) -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_threads)
            .thread_name(thread_name)
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap()
    }

    fn new_timer(wait_duration: i64) -> Timer {
        Timer::new(wait_duration)
    }
}