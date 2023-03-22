use tokio::{sync::{Mutex, broadcast::{Receiver, Sender}}};
use std::{path::PathBuf, collections::{hash_map::DefaultHasher, HashSet}, sync::Arc};
use notify::{Event, event::ModifyKind, EventKind};
use std::{hash::{Hash,Hasher}};
use crate::logger::{r#struct::Logger,debug::DebugLogging};
use super::{init::Watcher, watcher_errors::path_error::PathError, watcher_scripts::{WatcherScripts, Script}};

impl Watcher {

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

    pub fn ignore(event: &notify::Event) -> bool {
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

    fn handle_modifying_event(
        event: Event,
        root_dir: PathBuf,
        scripts: WatcherScripts,
        subscribe_channel: Sender<(PathBuf, Vec<Script>)>
    ) -> () {
        Logger::log_debug_string(&format!("not ignoring event of kind: {:?}", &event.kind));
        let scripts_clone = scripts.clone();
        let event_scripts = match scripts_clone.scripts_by_event_triggers.get(&event.kind) { 
            Some(scripts) => scripts,
            None => return ()
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
        
        for event_home_dir in unique_event_home_dirs {
            let _ = subscribe_channel.send((event_home_dir, event_scripts.clone()));
        }
    }
 
    pub async fn watch_events(
        arc_clone:&Arc<Mutex<tokio::runtime::Runtime>>,
        mut events_receiver: Receiver<Result<Event, Arc<notify::Error>>>,
        root_dir: PathBuf,
        scripts: WatcherScripts,
        subscribe_channel: Sender<(PathBuf, Vec<Script>)>
    ) -> () {
        let runtime_arc = arc_clone.lock().await;
        let events_task = runtime_arc.spawn(async move {
            while let Ok(res) = events_receiver.recv().await {
                match res {
                    Ok(event) => {
                        match Self::ignore(&event) {
                            true => {},
                            false => {
                                Logger::log_debug_string(&format!("not ignoring event of kind: {:?}", &event.kind));
                                Self::handle_modifying_event(
                                    event, 
                                    root_dir.clone(), 
                                    scripts.clone(), 
                                    subscribe_channel.clone()
                                );
                            }
                        };
                    },
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });
        events_task.await;
    } 

}