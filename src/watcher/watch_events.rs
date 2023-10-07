use log::{debug, error};
use tokio::sync::TryLockError;
use std::{path::PathBuf, collections::HashSet};
use notify::{Event, event::ModifyKind, EventKind};
use crate::utilities::traits::Utilities;
use crate::scripts::structs::Scripts;
use super::structs::Watcher;
use crate::utilities::thread_types::{EventsReceiver, SubscribeSender};

impl Watcher {
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
                    error!(r#"error while looking for events root directory, i.e. the uppermost affected directory "prior" to the directory rusty-hooks is watching. skipping."#);
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
        debug!("spawned event watching thread");
        while let Ok(res) = events_receiver.recv().await {
            match res {
                Ok(event) => {
                    match Self::ignore(&event) {
                        true => {
                            // debug!(&format!("ignoring event of kind: {:?}", &event.kind));
                        },
                        false => {
                            debug!("not ignoring event of kind: {:?}", event.kind);
                            let unique_event_home_dirs = Self::get_unique_event_home_dirs(
                                &event,
                                root_dir.clone(),
                            );
                            for event_home_dir in unique_event_home_dirs {
                                match subscribe_channel.send((event_home_dir, scripts.get_by_event(&event.kind))) {
                                    Ok(_) => {
                                        debug!("successfuly sent new path to subscription thread");
                                        // debug!(&format!("num of sub receivers: {}", subscribe_channel.receiver_count()));
                                    },
                                    Err(e) => {
                                        error!("error while attempting to subscribe to new path: {}", e)
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
