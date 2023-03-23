use std::{path::PathBuf, time::{Duration}, collections::{HashMap, hash_map::DefaultHasher}, sync::{Arc, MutexGuard}};
use chrono::{DateTime, Utc};
use notify::Event;
use tokio::{sync::{broadcast::{Sender, Receiver}, Mutex},time::{sleep}};
use std::hash::Hash;
use std::hash::Hasher;
use crate::{logger::{r#struct::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging}};
use super::{watcher_scripts::{Script}, watcher_errors::{thread_error::ThreadError, timer_error::TimerError, path_error::PathError}};

pub struct PathSubscriber {
    pub paths: Arc<std::sync::Mutex<HashMap<u64, (PathBuf, Vec<Script>)>>>,
    pub subscribe_channel: (Sender<(PathBuf, Vec<Script>)>, Receiver<(PathBuf, Vec<Script>)>),
    pub unsubscribe_channel: (Sender<PathBuf>, Receiver<PathBuf>)
}

impl PathSubscriber {

    pub fn new() -> Self {
        let (path_subscriber, receive_new_subscription) = tokio::sync::broadcast::channel::<(PathBuf, Vec<Script>)>(16);
        let (path_unsubscriber, receive_path_desubscription) = tokio::sync::broadcast::channel::<PathBuf>(16);
        let paths_hash:HashMap<u64, (PathBuf, Vec<Script>)> = HashMap::new();
        let paths = Arc::new(std::sync::Mutex::new(paths_hash));
        PathSubscriber {
            paths,
            subscribe_channel: (path_subscriber, receive_new_subscription),
            unsubscribe_channel: (path_unsubscriber, receive_path_desubscription)
        }
    }

    pub fn hasher(path: &String) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(& mut hasher);
        hasher.finish()
    }

    pub fn unsubscribe(path: &PathBuf, mut paths: MutexGuard<'_, HashMap<u64, (PathBuf, Vec<Script>)>>) -> Result<(), PathError> {
        let path_string = path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());
        match paths.remove_entry(&path_hash) {
            Some(_) => Ok(()),
            None => Err(PathError::UnsubscribeError(format!("didn't find path in cache, didnt unsubscribe")))
        }
    }

    async fn time_to_break (timer_controller: &Arc<Mutex<(chrono::Duration,DateTime<Utc>)>>) -> Result<bool, TimerError> {
        let now = chrono::prelude::Utc::now();
        let controller = timer_controller.lock().await;
        // Logger::log_info_string(&format!("timestamp being waited on: {}", controller.1.to_rfc3339()));
        let from_then_to_now_option = now.signed_duration_since(controller.1);
        // evaluate duration of time from shared state AKA original timestamp to now, returning a duration of 0 if the calculation yields something funky
        // that funky yield only happens if var now is somehow behind the original timestamp, which isnt possible unless theres weird concurrent operations happening, in which case waiting for this method to be called again is fine anyways
        Ok(from_then_to_now_option > controller.0)
    }

    pub async fn timer<'a>(timer_controller: Arc<Mutex<(chrono::Duration,DateTime<Utc>)>>) -> Result<(), TimerError<'a>> {
        // receive a (duration, timestamp_to_depend_on) arc<mutex> tuple
        loop {
            let should_break = match Self::time_to_break(&timer_controller).await {
                Ok(should) => should,
                Err(e) => {
                    Logger::log_error_string(&e.to_string());
                    false
                }
            };
            if should_break { 
                break
            } else {
                sleep(Duration::from_millis(500)).await;
            }
        }
        return Ok(())
    }

    async fn start_waiting(original_path: PathBuf, mut events_listener: Receiver<Result<Event, Arc<notify::Error>>>) {
        // thread that waits for events at particular path to end based on 1 or 2min timer and returns once either the events receiver closes or the timer runs out
        let timer_controller = Arc::new(Mutex::new((
            chrono::Duration::seconds(10),
            chrono::prelude::Utc::now()
        )));
        let controller_0 = timer_controller.clone();
        let controller_1 = timer_controller.clone();

        let timer_thread = tokio::spawn(async move {
            Self::timer(controller_0).await
        });

        let events_thread = tokio::spawn(async move {
            let path_string = original_path.to_str().unwrap_or("unable to pull string out of path buf");
            let hashed_original_path = Self::hasher(&path_string.to_string());
            while let Ok(event) = events_listener.recv().await {
                // Logger::log_debug_string(&format!("waiting on events to cease at {}", path_string));
                let valid_event = match event {
                    Ok(event) => event,
                    Err(e) => { continue }
                };
                // Logger::log_debug_string(&format!("parent path: {}, hashed into: {}", path_string, hashed_original_path));
                let path_overlap = valid_event.paths.iter().fold(false, |overlap, cur_path| {
                    if overlap { return true };
                    let cur_path_parent = cur_path.ancestors().any(|ancestor| {
                        let path_string = ancestor.to_str().unwrap_or("unable to pull string out of path buf");
                        let ancestor_hash = Self::hasher(&path_string.to_string());
                        ancestor_hash == hashed_original_path
                    });
                    cur_path_parent
                });
                if path_overlap {
                    Logger::log_debug_string(&"attempting to update timestamp to wait duration from".to_string());
                    // need to update the timer's timestamp to now
                    let now = chrono::prelude::Utc::now();
                    let mut controller_lock = controller_1.lock().await;
                    controller_lock.1 = now;
                } else {
                    // continue to let the timer run out while monitoring new events
                    continue
                }
            }
        });

        match timer_thread.await {
            Ok(_) => {},
            Err(e) => {
                Logger::log_error_string(&e.to_string())
            }
        }
        // stop listening for events once the timer has run out
        events_thread.abort();
    }

    fn lock_and_update_paths<'a>(new_path:PathBuf, paths: Arc<std::sync::Mutex<HashMap<u64, (PathBuf, Vec<Script>)>>>, scripts: Vec<Script>) -> Result<bool, ThreadError<'a>> {
        let mut paths_lock = match paths.lock() {
            Ok(path) => path,
            Err(e) => {
                let poison_error_message = e.to_string();
                let message = format!("unable to lock onto watched paths structure whilst receiving new path subscription: {}", poison_error_message);
                Logger::log_error_string(&format!("{}", &message));
                let thread_error = ThreadError::PathsLockError(e);
                // handle the poison error better here  - https://users.rust-lang.org/t/mutex-poisoning-why-and-how-to-recover/72192/12#:~:text=You%20can%20ignore%20the%20poisoning%20by%20turning,value%20back%20into%20a%20non%2Dbroken%20state.
                // TODO: implement a path cache, so that in the event of a poison error the path is reset to the cache?
                return Ok(false)
            }
        };
    
        let path_string = new_path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());
        
        // should only return true if the path isn't found in the datastructure
        let should_add_path = paths_lock.get(&path_hash).is_none();

        paths_lock.insert(path_hash, (new_path, scripts));

        Ok(should_add_path)
    }

    pub async fn route_subscriptions(&self, events_listener: Sender<Result<Event, Arc<notify::Error>>>, spawn_channel: Sender<(PathBuf, Vec<Script>)>) -> () {
        let mut subscription_listener = self.subscribe_channel.0.subscribe();
        let paths = self.paths.clone();
        let wait_threads = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("timer-threads")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        while let Ok((path, scripts)) = subscription_listener.recv().await {
            let events = events_listener.subscribe();
            let subscribed_to_new_path = match Self::lock_and_update_paths(path.clone(), paths.clone(), scripts.clone()) {
                Ok(subscribed) => subscribed,
                Err(e) => {
                    return ();
                }
            };
            let path_str = path.to_str().unwrap_or("unable to read incoming path into string");
            match subscribed_to_new_path {
                true => {
                    Logger::log_info_string(&format!("watching new path at {}",path_str));
                },
                false => {
                    Logger::log_info_string(&format!("received new path subscription, but it's already being observed {}", path_str));
                }
            }
            if subscribed_to_new_path {
                let spawn_channel = spawn_channel.clone();
                wait_threads.spawn(async move {
                    Self::start_waiting(path.clone(), events).await;
                    Logger::log_info_string(&"successfully waited on timer expiration, now running scripts".to_string());
                    let stuff_to_send = (path.clone(), scripts);
                    match spawn_channel.send(stuff_to_send) {
                        Ok(_) => { Logger::log_debug_string(&"sent path and scripts to script runner over spawn channel".to_string())},
                        Err(e) => {
                            Logger::log_error_string(&e.to_string())
                        }
                    }
                });
            }
        }

        wait_threads.shutdown_timeout(Duration::from_secs(30));
    
    }
    }