use tokio::{sync::{Mutex, broadcast::Sender, TryLockError}, task::JoinHandle};
use std::{sync::Arc, path::PathBuf};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};
use std::path::Path;
use crate::logger::{structs::Logger, info::InfoLogging, debug::DebugLogging, error::ErrorLogging};
use crate::scripts::structs::{Scripts, Script};
use crate::errors::watcher_errors::{watcher_error::WatcherError};
use crate::utilities::{traits::Utilities, thread_types::{EventChannel, BroadcastSender}};
use crate::runner::types::SpawnMessage;
use super::structs::{PathSubscriber, Watcher};

impl Watcher {
    pub fn new() -> Result<Self, WatcherError> {
        let watcher_runtime = <Self as Utilities>::new_runtime(4, &"watcher-runtime".to_string())?;
        Ok(Watcher {
            runtime: Arc::new(Mutex::new(watcher_runtime)),
            subscriber: PathSubscriber::new(),
        })
    }

    pub async fn start(&self, spawn_channel: Sender<(PathBuf, Vec<Script>)>, watch_path: String, scripts: &Scripts) -> Result<(), notify::Error>{
        let scripts_clone = scripts.clone();
        Self::watch_handler(&self, self.runtime.clone(), spawn_channel,  watch_path, scripts_clone).await
    }
    
    fn notifier_task() -> notify::Result<(
        RecommendedWatcher, EventChannel
    )> {
        let (events_channel, broadcast_rx) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let events_channel_clone = events_channel.clone();

        let watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(r) => {
                    match events_channel.send(Ok(r)) {
                        Ok(_) => {},
                        Err(e) => {
                            Logger::log_debug_string(&format!("{} - num existing receivers: {}", e.to_string(), events_channel.receiver_count()))
                        }
                    };
                }
                Err(e) => {
                    let error_arc = Arc::new(e);
                    events_channel.send(Err(error_arc)).unwrap();
                }
            }
        }, Config::default())?;
    
        Ok((watcher, (events_channel_clone, broadcast_rx)))
    }
    
    async fn watch_handler<P: AsRef<Path>>(
        &self,
        runtime_arc: Arc<Mutex<tokio::runtime::Runtime>>, 
        spawn_channel: BroadcastSender<SpawnMessage>,
        root_watch_path: P, 
        scripts: Scripts
    ) -> notify::Result<()> {
        let root_dir =root_watch_path.as_ref().to_path_buf();
        let (mut notifier_handle, (broadcast_sender, events)) = Self::notifier_task()?;
        notifier_handle.watch(root_watch_path.as_ref(), RecursiveMode::Recursive)?;
        
        // let (paths_subscriber_clone_1, paths_subscriber_clone_2) = (self.subscriber.clone(), self.subscriber.clone());
        let subscriber_channel_1 = self.subscriber.subscribe_channel.0.clone();
        let subscriber_channel_2 = self.subscriber.subscribe_channel.0.clone();
        let unsubscribe_channel_1 = self.subscriber.unsubscribe_channel.0.clone();
        let paths_clone_1 = self.subscriber.paths.clone();
        let paths_clone_2 = self.subscriber.paths.clone();

        let runtime_arc = match runtime_arc.try_lock() {
            Ok(r) => r,
            Err(e) => {
                Logger::log_info_string(&format!("{}", e.to_string()));
                return Ok(())
            }
        };
        
        let unsubscribe_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned unsubscribe thread".to_string());
            PathSubscriber::unsubscribe_task(unsubscribe_channel_1, paths_clone_1).await;
        });
        
        let event_channel_for_path_subscriber = broadcast_sender.clone();
        // start watching for new path subscriptions coming from the event watcher
        let subscription_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned subscribe thread".to_string());
            match PathSubscriber::route_subscriptions(
                event_channel_for_path_subscriber,
                spawn_channel,
                subscriber_channel_1,
                paths_clone_2
            ).await {
                Ok(_) => {},
                Err(e) => {
                    Logger::log_error_string(&format!("error while managing incoming subscriptions: {}", e.to_string()))
                }
            }
        });

        // start watching for new events from the notify crate
        let events_task:JoinHandle<Result<(), TryLockError>> = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned event watching thread".to_string());
            Self::watch_events(
                events, 
                root_dir,
                scripts.clone(), 
                subscriber_channel_2
            ).await
        });

        Self::handle_all_futures(events_task, subscription_task, unsubscribe_task).await;

        // cleanup
        notifier_handle.unwatch(root_watch_path.as_ref())?;

        Ok(())
    }
    // kills all tasks if any exit
    async fn handle_all_futures(events: JoinHandle<Result<(), TryLockError>>, subscriptions:JoinHandle<()>, unsubscription: JoinHandle<()>) -> () {
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
        Logger::log_info_string(&"when one task exits all are killed and the program exits".to_string())
    }
}
