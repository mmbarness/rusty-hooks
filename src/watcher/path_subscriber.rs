use std::{path::PathBuf, time::Duration, collections::HashMap, sync::Arc};
use crate::{runner::types::SpawnMessage};
use crate::utilities::{thread_types::{BroadcastReceiver, EventMessage, BroadcastSender}, traits::Utilities};
use crate::logger::{r#struct::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging};
use super::watcher_scripts::Script;
use super::structs::PathSubscriber;
use super::watcher_errors::{thread_error::ThreadError,path_error::PathError};
use super::types::{PathHash, PathsCache};


impl PathSubscriber {
    pub fn new() -> Self {
        let path_cache:HashMap<PathHash, (PathBuf, Vec<Script>)> = HashMap::new();
        let paths = Arc::new(std::sync::Mutex::new(path_cache));
        PathSubscriber {
            paths,
            subscribe_channel: Self::new_channel::<SpawnMessage>(),
            unsubscribe_channel: Self::new_channel::<PathBuf>(),
        }
    }

    pub fn unsubscribe(path: &PathBuf, mut paths: PathsCache<'_>) -> Result<(), PathError> {
        let path_string = path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());
        match paths.remove_entry(&path_hash) {
            Some(_) => Ok(()),
            None => Err(PathError::UnsubscribeError(format!("didn't find path in cache, didnt unsubscribe")))
        }
    }

    pub async fn unsubscribe_task(&self) -> () {
        while let unvalidated_path = self.unsubscribe_channel.0.subscribe().recv().await {
            match unvalidated_path {
                Ok(path) => {
                    let paths = self.paths.clone();
                    let paths = match paths.lock() {
                        Ok(p) => p,
                        Err(e) => {
                            Logger::log_error_string(&format!("unable to lock onto paths while trying to unsubscribe: {:?}", e.to_string()));
                            continue;
                        }
                    };
                    match PathSubscriber::unsubscribe(&path, paths) {
                        Ok(_) => {
                            let path_display = path.display();
                            let unsubscribe_success_message = &format!("successfully unsubscribed from path: {}", path_display);
                            Logger::log_info_string(unsubscribe_success_message)
                        },
                        Err(e) => {
                            Logger::log_error_string(&format!("{}", e.to_string()))
                        }
                    }
                },
                Err(e) => {
                    Logger::log_error_string(&e.to_string());
                    break;
                },
            }
        }
    }

    async fn start_waiting(original_path: PathBuf, mut events_listener: BroadcastReceiver<EventMessage>) {
        // thread that waits for events at particular path to end based on 1 or 2min timer and returns once either the events receiver closes or the timer runs out
        let new_timer = Self::new_timer(10);
        let timer_controller = new_timer.controller.clone();

        let timer_thread = tokio::spawn(async move {
            new_timer.wait().await
        });

        let events_thread = tokio::spawn(async move {
            let path_string = original_path.to_str().unwrap_or("unable to pull string out of path buf");
            let hashed_original_path = Self::hasher(&path_string.to_string());
            while let Ok(event) = events_listener.recv().await {
                let valid_event = match event {
                    Ok(event) => event,
                    Err(e) => { continue }
                };
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
                    // need to update the timer's timestamp to now
                    let now = chrono::prelude::Utc::now();
                    let mut controller_lock = timer_controller.lock().await;
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

    fn lock_and_update_paths<'a>(new_path:PathBuf, paths: Arc<std::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>, scripts: Vec<Script>) -> Result<bool, ThreadError<'a>> {
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

    pub async fn route_subscriptions(&self, events_listener: BroadcastSender<EventMessage>, spawn_channel: BroadcastSender<SpawnMessage>) -> () {
        let mut subscription_listener = self.subscribe_channel.0.subscribe();
        let paths = self.paths.clone();
        let wait_threads = Self::new_runtime(4, &"timer-threads".to_string());
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
                // TODO: create channel for a timer and event thread to communicate, and spawn both on wait threads so that start_waiting need not spawn its own
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

        wait_threads.shutdown_timeout(Duration::from_secs(10));
    
    }
    }