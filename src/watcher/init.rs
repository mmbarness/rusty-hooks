use tokio::{sync::{Mutex, broadcast::Sender}, runtime::{Builder, Runtime}};
use std::{sync::Arc, path::PathBuf};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};
use std::{path::Path};
use crate::logger::{r#struct::Logger, info::InfoLogging, error::ErrorLogging, debug::DebugLogging};
use super::{watcher_scripts::{WatcherScripts, Script}, path_subscriber::PathSubscriber};
use super::{path_subscriber};

#[derive(Debug)]
pub struct Watcher {
    pub runtime: Arc<Mutex<Runtime>>
}

impl Watcher {
    pub fn new() -> Self {
        let watcher_runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("watcher-runtime")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        let runtime_arc = Arc::new(Mutex::new(watcher_runtime));
        Watcher {
            runtime: runtime_arc
        }
    }

    pub async fn start(&self, spawn_channel: Sender<(PathBuf, Vec<Script>)>, watch_path: String, scripts: &WatcherScripts) -> Result<(), notify::Error>{
        let scripts_clone = scripts.clone();
        Self::watch_handler(self.runtime.clone(), spawn_channel,  watch_path, scripts_clone).await
    }
    
    fn run_watcher() -> notify::Result<(
        RecommendedWatcher, (
            tokio::sync::broadcast::Sender<Result<Event, Arc<notify::Error>>>,
            tokio::sync::broadcast::Receiver<Result<Event, Arc<notify::Error>>>
        )
    )> {
        let (events_channel, broadcast_rx) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let events_channel_clone = events_channel.clone();

        let watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(r) => {
                    match events_channel.send(Ok(r)) {
                        Ok(_) => {
                            Logger::log_debug_string(&format!("successfully sent new event- num existing receivers {}", events_channel.receiver_count()));
                        },
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

        Logger::log_info_string(&"beginning to watch for events".to_string());
    
        Ok((watcher, (events_channel_clone, broadcast_rx)))
    }
    
    async fn watch_handler<P: AsRef<Path>>(
        runtime_arc: Arc<Mutex<tokio::runtime::Runtime>>, 
        spawn_channel: Sender<(PathBuf, Vec<Script>)>,
        root_watch_path: P, 
        scripts: WatcherScripts
    ) -> notify::Result<()> {
        let (mut watcher, (broadcast_sender, events)) = Self::run_watcher()?;

        let event_channel_for_path_subscriber = broadcast_sender.clone();

        watcher.watch(root_watch_path.as_ref(), RecursiveMode::Recursive)?;

        let arc_clone = runtime_arc.clone();
        
        let paths_subscriber = path_subscriber::PathSubscriber::new();
        let subscribe_channel = paths_subscriber.subscribe_channel.0.clone();
        let unsubscribe_channel = paths_subscriber.unsubscribe_channel.0.clone();

        let paths_subscriber_arc = Arc::new(tokio::sync::Mutex::new(paths_subscriber));
        let paths_subscriber_clone = paths_subscriber_arc.clone();
        
        let runtime_arc = match arc_clone.try_lock() {
            Ok(r) => {
                Logger::log_info_string(&"locked onto runtime".to_string());
                r
            },
            Err(e) => {
                Logger::log_info_string(&format!("{}", e.to_string()));
                return Ok(())
            }
        };
        
        let unsubscribe_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned unsubscribe thread".to_string());
            let local_path_subscriber = paths_subscriber_clone.lock().await;
            while let unvalidated_path = unsubscribe_channel.subscribe().recv().await {
                match unvalidated_path {
                    Ok(path) => {
                        let paths = local_path_subscriber.paths.clone();
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
                    }
                }
            }
        });

        // start watching for new path subscriptions, coming from the other thread spawned onto the runtime
        let subscription_task = runtime_arc.spawn(async move {
            Logger::log_debug_string(&"spawned subscribe thread".to_string());
            let local_subscriber = paths_subscriber_arc.clone();
            let paths_subscriber_lock = local_subscriber.lock().await;
            Logger::log_info_string(&"watching for new path subscriptions".to_string());
            paths_subscriber_lock.route_subscriptions(event_channel_for_path_subscriber, spawn_channel).await;
        });
        
        let root_dir =root_watch_path.as_ref().to_path_buf();
        // start watching for new events from the notify crate
        let events_task = runtime_arc.spawn(async move {
            let task = Self::watch_events(
                events, 
                root_dir,
                scripts.clone(), 
                subscribe_channel
            ).await;
            return task
        });

        tokio::select! {
            a = events_task => {
                match a {
                    Ok(_) => {

                    },
                    Err(e) => {
                        Logger::log_debug_string(&format!("events_task failed: {}, exiting", &e.to_string()));
                    }
                }
            },
            b = subscription_task => {
                match b {
                    Ok(_) => {

                    },
                    Err(e) => {
                        Logger::log_debug_string(&format!("subscription_task failed: {}, exiting", &e.to_string()));
                    }
                }
            },
            c = unsubscribe_task => {
                match c {
                    Ok(_) => {

                    },
                    Err(e) => {
                        Logger::log_debug_string(&format!("unsubscription_task failed: {}, exiting", &e.to_string()));
                    }
                }
            }
        }

        Ok(())
    }
}
