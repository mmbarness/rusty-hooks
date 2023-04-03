use tokio::{sync::{Mutex, broadcast::Sender, TryLockError}, task::JoinHandle};
use std::{sync::Arc, path::PathBuf};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};
use crate::{logger::{structs::Logger, info::InfoLogging, debug::DebugLogging}, utilities::thread_types::UnsubscribeSender, errors::{watcher_errors::{event_error::EventError, subscriber_error::SubscriberError}, runtime_error::enums::RuntimeError}};
use crate::scripts::structs::{Scripts, Script};
use crate::errors::watcher_errors::{watcher_error::WatcherError};
use crate::utilities::{traits::Utilities, thread_types::{EventChannel}};
use super::structs::{PathSubscriber, Watcher};

impl Watcher {
    pub fn new() -> Result<Self, WatcherError> {
        let watcher_runtime = <Self as Utilities>::new_runtime(4, &"watcher-runtime".to_string())?;
        Ok(Watcher {
            runtime: Arc::new(Mutex::new(watcher_runtime)),
            subscriber: PathSubscriber::new(),
        })
    }

    fn notifier_task() -> notify::Result<(
        RecommendedWatcher, EventChannel
    )> {
        let (events_channel, broadcast_rx) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let events_channel_clone = events_channel.clone();

        let watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(r) => {
                    let _ = events_channel.send(Ok(r));
                }
                Err(e) => {
                    let error_arc = Arc::new(e);
                    events_channel.send(Err(error_arc)).unwrap();
                }
            }
        }, Config::default())?;
    
        Ok((watcher, (events_channel_clone, broadcast_rx)))
    }
    
    pub async fn start(
        &self, spawn_channel: Sender<(PathBuf, Vec<Script>)>, unsubscribe_channel: UnsubscribeSender, watch_path: PathBuf, scripts: &Scripts
    ) -> Result<(), WatcherError> {
        Logger::log_info_string(&format!("now watching path: {}", &watch_path.to_str().unwrap()));
        let watch_path_clone_1 = watch_path.clone();
        let watch_path_clone_2 = watch_path.clone();
        let scripts_clone = scripts.clone();
        let runtime_clone = self.runtime.clone();
        let (mut notifier_handle, (broadcast_sender, events)) = Self::notifier_task().map_err(EventError::NotifyError)?;
        notifier_handle.watch(watch_path.as_ref(), RecursiveMode::Recursive).map_err(EventError::NotifyError)?;

        let subscriber_channel_1 = self.subscriber.subscribe_channel.0.clone();
        let subscriber_channel_2 = self.subscriber.subscribe_channel.0.clone();
        let paths_clone_1 = self.subscriber.paths.clone();
        let paths_clone_2 = self.subscriber.paths.clone();

        let unsubscribe_receiver = unsubscribe_channel.subscribe();

        let runtime_arc = runtime_clone.try_lock().map_err(RuntimeError::LockError)?;
        
        let unsubscribe_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned unsubscribe thread".to_string());
            PathSubscriber::unsubscribe_task(unsubscribe_receiver, paths_clone_1, watch_path_clone_2).await
        });
        
        let event_channel_for_path_subscriber = broadcast_sender.clone();
        // start watching for new path subscriptions coming from the event watcher
        let subscription_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned subscribe thread".to_string());
            PathSubscriber::route_subscriptions(
                event_channel_for_path_subscriber,
                spawn_channel,
                subscriber_channel_1,
                paths_clone_2
            ).await
        });

        // start watching for new events from the notify crate
        let events_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned event watching thread".to_string());
            Self::watch_events(
                events, 
                watch_path_clone_1,
                scripts_clone, 
                subscriber_channel_2
            ).await
        });

        Self::handle_all_futures(events_task, subscription_task, unsubscribe_task).await;

        // cleanup
        notifier_handle.unwatch(watch_path.as_ref()).map_err(EventError::NotifyError)?;

        Ok(())
    }
    // kills all tasks if any exit
    async fn handle_all_futures(events: JoinHandle<Result<(), TryLockError>>, subscriptions:JoinHandle<Result<(), SubscriberError>>, unsubscription: JoinHandle<Result<(), SubscriberError>>) -> () {
            tokio::select! {
                a = events => {
                    match a {
                        Ok(_) => { 
                            Logger::log_debug_string(&"events_task exited".to_string());
                        },
                        Err(e) => {
                            Logger::log_debug_string(&format!("events_task failed: {}, exiting", &e.to_string()));
                        }
                    }
                },
                b = subscriptions => {
                    match b {
                        Ok(_) => {
                            Logger::log_debug_string(&"subscription_task exited".to_string());
                        },
                        Err(e) => {
                            Logger::log_debug_string(&format!("subscription_task failed: {}, exiting", &e.to_string()));
                        }
                    }
                },
                c = unsubscription => {
                    match c {
                        Ok(_) => {
                            Logger::log_debug_string(&"unsubscribe_task exited".to_string());
                        },
                        Err(e) => {
                            Logger::log_debug_string(&format!("unsubscription_task failed: {}, exiting", &e.to_string()));
                        }
                    }
                }
            }
        Logger::log_info_string(&"when one task exits for whatever reason (probably an error), all are killed and the program exits".to_string())
    }
}
