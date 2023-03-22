use tokio::{sync::{Mutex}, runtime::{Builder}};
use std::{path::PathBuf, collections::{hash_map::DefaultHasher, HashSet}, sync::Arc, time::Duration};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config, event::ModifyKind, EventKind};
use std::{hash::{Hash,Hasher}, path::Path};
use log::{info, error};
use crate::logger::{r#struct::Logger, info::InfoLogging, error::ErrorLogging, debug::DebugLogging};

use super::{watcher_scripts::WatcherScripts, watcher_errors::{path_error::PathError}, runner::Runner};
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
            .thread_name("my-custom-name")
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

    fn ignore(event: &notify::Event) -> bool {
        match &event.kind {
            EventKind::Modify(e) => {
                match e {
                    // type of event that takes place when the file finishes and its .tmp extension is removed
                    ModifyKind::Name(notify::event::RenameMode::To) => false,
                    _ => true,
                }
            },
            _ => true,
        }
    }

    
    pub fn hasher(path: &PathBuf) -> Result<u64, PathError> {
        let mut hasher = DefaultHasher::new();
        let abs_path = path.canonicalize()?;
        abs_path.hash(& mut hasher);
        Ok(hasher.finish())
    }
    
    fn walk_up_to_event_home_dir<'a>(leaf:PathBuf, root: PathBuf) -> Result<PathBuf,PathError> {
        // walk upwards from the leaf until we hit the parentmost directory of the event, i.e. the path that is one level below the root
        let leaf_hash = Self::hasher(&leaf)?;
        let parent_of_leaf = leaf.parent().ok_or(PathError::TraversalError)?;
        let parent_of_leaf_hash = Self::hasher(&parent_of_leaf.to_path_buf())?;
        let root_hash = Self::hasher(&root)?;
        if parent_of_leaf_hash == root_hash {
            return Ok(leaf)
        } else if leaf_hash == root_hash {
            return Err(PathError::TraversalError)
        } else {
            Self::walk_up_to_event_home_dir(parent_of_leaf.to_path_buf(), root)
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
            // multi_sender.send(res);
            // futures::executor::block_on(async {
            //     tx.send(res).await.unwrap();
            // })
        }, Config::default())?;

        Logger::log_info_string(&"beginning to watch for events".to_string());
    
        Ok((watcher, (broadcast_clone, broadcast_rx)))
    }
    
    async fn watch_handler<P: AsRef<Path>>(runtime_arc: &Arc<Mutex<tokio::runtime::Runtime>>, root_watch_path: P, scripts: WatcherScripts) -> notify::Result<()> {
        let (mut watcher, (broadcast_sender, mut events)) = Self::run_watcher()?;

        let event_channel_for_path_subscriber = broadcast_sender.clone();

        watcher.watch(root_watch_path.as_ref(), RecursiveMode::Recursive)?;

        let arc_clone = runtime_arc.clone();

        let script_runner = Runner::new();
        let subscribe_channel_0 = script_runner.spawn_channel.0.clone();
        let subscribe_channel_1 = script_runner.spawn_channel.0.clone();

        let paths_subscriber = path_subscriber::PathSubscriber::new();
        let paths_subscriber_arc = Arc::new(tokio::sync::Mutex::new(paths_subscriber));

        let runtime_arc = arc_clone.lock().await;

        // start watching for new path subscriptions, coming from the other thread spawned onto the runtime
        let subscription_task = runtime_arc.spawn(async move {
            let local_subscriber = paths_subscriber_arc.clone();
            let paths_subscriber_lock = local_subscriber.lock().await;
            paths_subscriber_lock.route_subscriptions(event_channel_for_path_subscriber, subscribe_channel_0).await;
        });
        
        let root_dir =root_watch_path.as_ref().to_path_buf();
        // start watching for new events from the notify crate
        let events_task = runtime_arc.spawn(async move {
            while let Ok(res) = events.recv().await {
                match res {
                    Ok(event) => {
                        match Self::ignore(&event) {
                            true => {
                                // Logger::log_debug_string(&format!("ignoring event of kind: {:?}", &event.kind))
                            },
                            false => {
                                Logger::log_debug_string(&format!("not ignoring event of kind: {:?}", &event.kind));
                                let scripts_clone = scripts.clone();
                                let event_scripts = match scripts_clone.scripts_by_event_triggers.get(&event.kind) { 
                                    Some(scripts) => scripts,
                                    None => continue
                                };
                                let root_dir = root_dir.clone();
                                let event_clone = event.clone();
                                let paths = event_clone.paths;
                                let acc: HashSet<PathBuf> = HashSet::new();
                                // convert to hashset to enforce unique values
                                let unique_event_home_dirs = paths.iter().fold(acc, |mut acc:HashSet<PathBuf>, path| {
                                    let events_root_dir = match Self::walk_up_to_event_home_dir(path.clone(), root_dir.clone()) {
                                        Ok(event_root) => event_root,
                                        Err(e) => {
                                            return acc
                                        }
                                    };
                                    acc.insert(events_root_dir.clone()); // returns true  or false based on whether or not it already existed, but we dont care
                                    acc
                                });
                                let dirs_string = match serde_json::to_string_pretty(&unique_event_home_dirs) {
                                    Ok(string) => string,
                                    Err(e) => e.to_string()
                                };
                                // Logger::log_debug_string(&format!("unique dirs hashset is length: {}", unique_event_home_dirs.len()));
                                // Logger::log_debug_string(&dirs_string);
                                
                                for event_home_dir in unique_event_home_dirs {
                                    let _ = subscribe_channel_1.send((event_home_dir, event_scripts.clone()));
                                }
                            }
                        };
                    },
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });

        subscription_task.await;
        
        events_task.await;

        let last_sender = broadcast_sender.clone();

        Ok(())
    }
}
