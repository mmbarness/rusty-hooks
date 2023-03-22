use tokio::{sync::{Mutex}, runtime::{Builder}};
use std::{sync::Arc};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};
use std::{path::Path};
use log::{info, error};
use crate::logger::{r#struct::Logger, info::InfoLogging, error::ErrorLogging};
use super::{watcher_scripts::{WatcherScripts}, runner::Runner};
use super::{path_subscriber};

#[derive(Debug)]
pub struct Watcher {
    pub watch_handle: Result<tokio::task::JoinHandle<()>, ()>
}

impl Watcher {
    pub fn init(scripts: &WatcherScripts) -> Self{
        let path = std::env::args()
            .nth(2)
            .expect("Argument 1 needs to be a path");
        info!("watching {}", path);
        let scripts_clone = scripts.clone();
        let tokio_runtime = Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("watcher-runtime")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        let runtime_arc = Arc::new(Mutex::new(tokio_runtime));
        let watch_handle = tokio::spawn(async move {
            Logger::log_info_string(&"spawned event watching thread".to_string());
            if let Err(e) = Self::watch_handler(&runtime_arc, path, scripts_clone).await {
                error!("error awaiting watch_handler src/watcher/init.rs:37 : {:?}", e)
            }
        });
        Watcher {
            watch_handle: Ok(watch_handle),
        }
    }
    
    fn run_watcher() -> notify::Result<(
        RecommendedWatcher, (
            tokio::sync::broadcast::Sender<Result<Event, Arc<notify::Error>>>,
            tokio::sync::broadcast::Receiver<Result<Event, Arc<notify::Error>>>
        )
    )> {
        let (broadcast_tx, broadcast_rx) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let broadcast_clone = broadcast_tx.clone();

        let watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(r) => {
                    match broadcast_tx.send(Ok(r)) {
                        Ok(_) => {
                            // Logger::log_info_string(&format!("successfully sent new event- num existing receivers {}", broadcast_tx.receiver_count()));
                        },
                        Err(e) => {
                            Logger::log_error_string(&format!("{} - num existing receivers: {}", e.to_string(), broadcast_tx.receiver_count()))
                        }
                    };
                }
                Err(e) => {
                    let error_arc = Arc::new(e);
                    broadcast_tx.send(Err(error_arc)).unwrap();
                }
            }

        }, Config::default())?;

        Logger::log_info_string(&"beginning to watch for events".to_string());
    
        Ok((watcher, (broadcast_clone, broadcast_rx)))
    }
    
    async fn watch_handler<P: AsRef<Path>>(
        runtime_arc: &Arc<Mutex<tokio::runtime::Runtime>>, 
        root_watch_path: P, 
        scripts: WatcherScripts
    ) -> notify::Result<()> {
        let (mut watcher, (broadcast_sender, mut events)) = Self::run_watcher()?;

        let event_channel_for_path_subscriber = broadcast_sender.clone();

        watcher.watch(root_watch_path.as_ref(), RecursiveMode::Recursive)?;

        let arc_clone = runtime_arc.clone();

        let script_runner = Runner::new();
        let spawn_channel = script_runner.spawn_channel.0.clone();
        
        let paths_subscriber = path_subscriber::PathSubscriber::new();
        let subscribe_channel = paths_subscriber.subscribe_channel.0.clone();
        let unsub_channel = paths_subscriber.unsubscribe_channel.0.clone();
        let paths_subscriber_arc = Arc::new(tokio::sync::Mutex::new(paths_subscriber));

        let runtime_arc = arc_clone.lock().await;

        // start watching for new path subscriptions, coming from the other thread spawned onto the runtime
        let subscription_task = runtime_arc.spawn(async move {
            let local_subscriber = paths_subscriber_arc.clone();
            let paths_subscriber_lock = local_subscriber.lock().await;
            paths_subscriber_lock.route_subscriptions(event_channel_for_path_subscriber, spawn_channel).await;
        });
        
        let runner_task = runtime_arc.spawn(async move {
            script_runner.init(unsub_channel).await
        });
        
        let root_dir =root_watch_path.as_ref().to_path_buf();
        // start watching for new events from the notify crate
        let events_task = Self::watch_events(
            &arc_clone,
            events, 
            root_dir,
            scripts.clone(), 
            subscribe_channel
        );

        subscription_task.await;
        
        events_task.await;

        runner_task.await;

        Ok(())
    }
}
