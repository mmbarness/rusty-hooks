use super::structs::Watcher;
use crate::scripts::structs::Scripts;
use crate::utilities::thread_types::{EventsReceiver, SubscribeSender};
use crate::{scripts::structs::Script, utilities::traits::Utilities};
use itertools::Itertools;
use log::{debug, error};
use notify::{event::ModifyKind, Event, EventKind};
use std::sync::Arc;
use std::{collections::HashSet, path::PathBuf};
use tokio::sync::broadcast::error::{RecvError, SendError};

impl Watcher {
    /// Awaits events emitted by notify. See [`notify::event`].
    pub async fn watch_events(
        mut events_receiver: EventsReceiver,
        root_dir: PathBuf,
        scripts: Scripts,
        subscribe_channel: SubscribeSender,
    ) -> Result<(), RecvError> {
        debug!("spawned event watching thread");
        loop {
            match events_receiver.recv().await {
                Ok(res) => {
                    Self::evaluate_event(res, &root_dir, &subscribe_channel, &scripts);
                }
                Err(e) => {
                    error!("Error encountered while a receiving a new event: {:?}", e);
                }
            };
        }
    }

    fn evaluate_event(
        res: Result<Event, Arc<notify::Error>>,
        root_dir: &PathBuf,
        subscribe_channel: &SubscribeSender,
        scripts: &Scripts,
    ) {
        match res {
            Ok(event) => {
                let subscription_errors =
                    Self::decide_to_subscribe(&event, &root_dir, &subscribe_channel, &scripts);
                for error in &subscription_errors {
                    error!("{:?}", error)
                }
            }
            Err(e) => {
                error!("notify error: {:?}", e);
            }
        }
    }

    fn decide_to_subscribe(
        event: &Event,
        root_dir: &PathBuf,
        subscribe_channel: &SubscribeSender,
        scripts: &Scripts,
    ) -> Vec<tokio::sync::broadcast::error::SendError<(PathBuf, Vec<Script>)>> {
        match Self::ignore(&event) {
            true => vec![],
            false => {
                let unique_event_home_dirs =
                    Self::get_unique_event_home_dirs(&event, root_dir.clone());
                unique_event_home_dirs
                    .iter()
                    .map(|event_home_dir| {
                        Self::send_new_event(&event, event_home_dir, &scripts, &subscribe_channel)
                    })
                    .filter_map(|f| f.err())
                    .collect_vec()
            }
        }
    }

    /// Accepts events of kind Modify, finds *their* root dirs, i.e. the uppermost affected directory relative to the root watched path, and sends those to the subscribe runtime.
    /// Example: If a watch path derived from the user-provided scripts.yml is /home/user/script_1_watch_path, and the incoming event occurred
    /// at /home/user/script_1/very/very/very/nested, /home/user/script_1/very will be returned.
    fn get_unique_event_home_dirs(event: &Event, root_dir: PathBuf) -> HashSet<PathBuf> {
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

    /// Sends a new event to the PathSubscriber runtime, along with its related scripts (based on the directory in question)
    fn send_new_event(
        event: &Event,
        event_dir: &PathBuf,
        scripts: &Scripts,
        subscribe_channel: &SubscribeSender,
    ) -> Result<(), SendError<(PathBuf, Vec<Script>)>> {
        match subscribe_channel.send((event_dir.clone(), scripts.get_by_event(&event.kind))) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
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
            }
            _ => true,
        }
    }
}
