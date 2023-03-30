use std::error::Error;
use std::{path::PathBuf, time::Duration, collections::HashMap, sync::Arc};
use notify::Event;
use tokio::task::JoinHandle;

use crate::errors::runtime_error::enums::RuntimeError;
use crate::errors::watcher_errors::event_error::EventError;
use crate::errors::watcher_errors::subscriber_error::SubscriberError;
use crate::errors::watcher_errors::thread_error::UnexpectedAnyhowError;
use crate::errors::watcher_errors::timer_error::TimerError;
use crate::scripts::structs::Script;
use crate::runner::types::SpawnMessage;
use crate::utilities::{thread_types::{BroadcastReceiver, EventMessage, BroadcastSender}, traits::Utilities};
use crate::logger::{structs::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging};
use super::structs::PathSubscriber;
use crate::errors::watcher_errors::{thread_error::ThreadError,path_error::PathError};
use super::types::{PathHash, PathsCache, PathsCacheArc};

impl PathSubscriber {
    pub fn new() -> Self {
        let path_cache:HashMap<PathHash, (PathBuf, Vec<Script>)> = HashMap::new();
        let paths = Arc::new(tokio::sync::Mutex::new(path_cache));
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

    pub async fn unsubscribe_task(unsubscribe_channel: BroadcastSender<PathBuf>, paths: PathsCacheArc) -> Result<(), SubscriberError> {
        loop {
            let path = unsubscribe_channel.subscribe().recv().await.map_err(ThreadError::RecvError)?;
            let paths = match paths.try_lock() {
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
        }
    }

    fn validate_event_subscription(event:Result<Event, Arc<notify::Error>>, mut num_events_errors: i32) -> Result<(Event, i32), (SubscriberError, i32)> {
        match event {
            Ok(event) => Ok((event, num_events_errors)),
            Err(e) => {
                let notify_error:SubscriberError = match Arc::try_unwrap(e) {
                    Ok(error) => {
                        let event_error:EventError = error.into();
                        event_error.into()
                    },
                    Err(arc_error) => {
                        let unexpected_error = ThreadError::UnexpectedError(anyhow::Error::new(arc_error));
                        unexpected_error.into()
                    }
                };
                num_events_errors += 1;
                Logger::log_error_string(&format!("error receiving events while waiting on timer to expire: {}", notify_error.to_string()));
                Err((notify_error, num_events_errors))
             }  
        }
    }

    async fn start_waiting(original_path: PathBuf, mut events_listener: BroadcastReceiver<EventMessage>) -> Result<(), SubscriberError>{
        // thread that waits for events at particular path to end based on 1 or 2min timer and returns once either the events receiver closes or the timer runs out
        let new_timer = Self::new_timer(10);
        let timer_controller = new_timer.controller.clone();

        let timer_thread = tokio::spawn(async move {
            new_timer.wait().await
        });

        let events_thread:JoinHandle<Result<(), SubscriberError>> = tokio::spawn(async move {
            let path_string = original_path.to_str().unwrap_or("unable to pull string out of path buf");
            let hashed_original_path = Self::hasher(&path_string.to_string());
            let mut num_events_errors = 0;
            let mut last_error: Option<SubscriberError> = None;
            loop {
                let event = match events_listener.recv().await.map_err(|e| {
                    ThreadError::RecvError(e.into())
                }) {
                    Ok(e) => e,
                    Err(e) => {
                        num_events_errors += 1;
                        last_error = Some(e.into());
                        continue;
                    }
                };
                if num_events_errors > 5 {
                    let new_unexpected_error:ThreadError = ThreadError::new_unexpected_error(format!("error while waiting on events to run out at watched path {}", path_string));
                    return Err(last_error.unwrap_or(new_unexpected_error.into()).into())
                };
                let valid_event = match Self::validate_event_subscription(event, num_events_errors) {
                    Ok((event, num_errors)) => {
                        num_events_errors = num_errors;
                        event
                    },
                    Err((e, num_errors)) => {
                        num_events_errors = num_errors;
                        last_error = Some(e);
                        continue;
                    }
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
                    let mut controller_lock = timer_controller.try_lock()?;
                    controller_lock.1 = now;
                } else {
                    // continue to let the timer run out while monitoring new events
                    continue
                }
            }
        });

        match timer_thread.await.map_err(RuntimeError::JoinError)? {
            Ok(_) => {
                events_thread.abort();
                Ok(())
            },
            Err(e) => Err(e.into()),
        }
    }

    fn lock_and_update_paths(new_path:PathBuf, paths: Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>, scripts: Vec<Script>) -> Result<bool, SubscriberError> {

        let mut paths_lock = paths.try_lock()?;

        let path_string = new_path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());
        
        // should only return true if the path isn't found in the datastructure
        let should_add_path = paths_lock.get(&path_hash).is_none();

        paths_lock.insert(path_hash, (new_path, scripts));

        Ok(should_add_path)
    }

    pub async fn route_subscriptions(
        events_listener: BroadcastSender<EventMessage>,
        spawn_channel: BroadcastSender<SpawnMessage>,
        subscribe_channel: BroadcastSender<(PathBuf, Vec<Script>)>,
        paths: PathsCacheArc
    ) -> Result<(), SubscriberError> {
        let mut subscription_listener = subscribe_channel.subscribe();
        let wait_threads = Self::new_runtime(4, &"timer-threads".to_string())?;
        while let unvalidated_subscription = subscription_listener.recv().await {
            match unvalidated_subscription {
                Ok((path, scripts)) => {
                    let path_string = path.to_str().unwrap_or(&"bad path parse");
                    Logger::log_debug_string(&format!("new path: {}", path_string));

                    let events = events_listener.subscribe();
                    let subscribed_to_new_path = match Self::lock_and_update_paths(path.clone(), paths.clone(), scripts.clone()) {
                        Ok(subscribed) => subscribed,
                        Err(e) => {
                            return Ok(());
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
                },
                Err(e) => {
                    Logger::log_error_string(&format!("error while receiving new subscription: {}", e.to_string()))
                }
            }
        };

        Logger::log_debug_string(&"exiting subscription watcher:".to_string());

        wait_threads.shutdown_timeout(Duration::from_secs(10));

        Ok(())
    
    }
}