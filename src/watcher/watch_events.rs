use tokio::sync::{TryLockError};
use std::{path::PathBuf, collections::{hash_map::DefaultHasher, HashSet}};
use notify::{Event, event::ModifyKind, EventKind};
use std::hash::{Hash,Hasher};
use crate::errors::watcher_errors::path_error::PathError;
use crate::logger::{structs::Logger,debug::DebugLogging, error::ErrorLogging};
use crate::scripts::structs::Scripts;
use super::structs::Watcher;
use crate::utilities::thread_types::{EventsReceiver, SubscribeSender};

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

    // accepts events of kind Modify, finds *their* root dirs, i.e. the uppermost affected directory relative to the root watched path, and sends those to the subscribe runtime
    fn get_unique_event_home_dirs(
        event: &Event,
        root_dir: PathBuf,
    ) -> HashSet<PathBuf> {
        let root_dir = root_dir.clone();
        let event_clone = event.clone();
        let paths = event_clone.paths;
        let acc: HashSet<PathBuf> = HashSet::new();
        // convert to hashset to enforce unique values
        paths.iter().fold(acc, |mut acc:HashSet<PathBuf>, path| {
            let events_root_dir = match Self::walk_up_to_event_home_dir(path.clone(), root_dir.clone()) {
                Ok(event_root) => event_root,
                Err(_) => {
                    // TODO: cache errored paths to retry later?
                    Logger::log_error_string(&format!(r#"error while looking for events root directory, i.e. the uppermost affected directory "prior" to the directory rusty-hooks is watching. skipping."#));
                    return acc
                }
            };
            acc.insert(events_root_dir.clone()); // returns true  or false based on whether or not it already existed, but we dont care
            acc
        })
        
    }

    pub async fn watch_events(
        mut events_receiver: EventsReceiver,
        root_dir: PathBuf,
        scripts: Scripts,
        subscribe_channel: SubscribeSender
    ) -> Result<(), TryLockError> {
        while let Ok(res) = events_receiver.recv().await {
            match res {
                Ok(event) => {
                    match Self::ignore(&event) {
                        true => {
                            // Logger::log_debug_string(&format!("ignoring event of kind: {:?}", &event.kind));
                        },
                        false => {
                            Logger::log_debug_string(&format!("not ignoring event of kind: {:?}", &event.kind));
                            let unique_event_home_dirs = Self::get_unique_event_home_dirs(
                                &event, 
                                root_dir.clone(),
                            );
                            for event_home_dir in unique_event_home_dirs {
                                match subscribe_channel.send((event_home_dir, scripts.get_by_event(&event))) {
                                    Ok(_) => {
                                        Logger::log_debug_string(&"successfuly sent new path to subscription thread".to_string());
                                        // Logger::log_debug_string(&format!("num of sub receivers: {}", subscribe_channel.receiver_count()));
                                    },
                                    Err(e) => {
                                        Logger::log_error_string(&format!("error while attempting to subscribe to new path: {}", e))
                                    }
                                }
                            }
                        }
                    };
                },
                Err(e) => println!("watch error: {:?}", e),
            }
        }
        Ok(())
    } 

}