use crate::{
    errors::{
        shared_errors::thread_errors::{ThreadError, UnexpectedAnyhowError},
        watcher_errors::subscriber_error::SubscriptionError,
    },
    utilities::{
        thread_types::{BroadcastReceiver, EventMessage},
        traits::Utilities,
    },
};
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use std::{path::PathBuf, sync::Arc};
use tokio::{runtime::Handle, sync::Mutex, task::JoinHandle};

use super::structs::PathSubscriber;

impl PathSubscriber {
    /// Begins awaiting incoming path subscription, sent from [`Watcher::watch_events()`]. If an incoming event
    /// is attached to a path already subscribed to, it updates the timer running on a separate thread.
    pub fn event_loop(
        mut events_listener: BroadcastReceiver<EventMessage>,
        subscription_path: PathBuf,
        timer_controller: Arc<Mutex<(Duration, DateTime<Utc>)>>,
    ) -> JoinHandle<Result<(), SubscriptionError>> {
        let handle = Handle::current();

        handle.spawn(async move {
            let mut num_events_errors = 0;
            let mut last_error: Option<SubscriptionError> = None;
            let sub_path_str = subscription_path
                .to_str()
                .unwrap_or("unable to pull string out of path buf");
            let sub_path_hash = Self::hasher(&sub_path_str.to_string());
            loop {
                let event = match events_listener
                    .recv()
                    .await
                    .map_err(|e| ThreadError::RecvError(e.into()))
                {
                    Ok(e) => e,
                    Err(e) => {
                        num_events_errors += 1;
                        last_error = Some(e.into());
                        continue;
                    }
                };
                if num_events_errors > 5 {
                    let new_unexpected_error: ThreadError =
                        ThreadError::new_unexpected_error(format!(
                            "error while waiting on events to run out at watched path {}",
                            sub_path_str
                        ));
                    return Err(last_error.unwrap_or(new_unexpected_error.into()).into());
                };
                let valid_event = match Self::validate_event_subscription(event, num_events_errors)
                {
                    Ok((event, num_errors)) => {
                        num_events_errors = num_errors;
                        event
                    }
                    Err((e, num_errors)) => {
                        num_events_errors = num_errors;
                        last_error = Some(e);
                        continue;
                    }
                };
                let path_overlap = Self::paths_overlap(valid_event, sub_path_hash);
                if path_overlap {
                    // need to update the timer's timestamp to now
                    let now = chrono::prelude::Utc::now();
                    let mut controller_lock = timer_controller.try_lock()?;
                    controller_lock.1 = now;
                } else {
                    // continue to let the timer run out while monitoring new events
                    continue;
                }
            }
        })
    }

    /// Evaluates the paths associated with an [`notify::Event`] for any ancestors that overlap
    /// with the path already subscribed to.
    fn paths_overlap(event: notify::Event, sub_path_hash: u64) -> bool {
        event.paths.iter().fold(false, |overlap, cur_path| {
            if overlap {
                return true;
            };
            let cur_path_parent = cur_path.ancestors().any(|ancestor| {
                let ancestor_path_str = ancestor
                    .to_str()
                    .unwrap_or("unable to pull string out of path buf");
                let ancestor_hash = Self::hasher(&ancestor_path_str.to_string());
                ancestor_hash == sub_path_hash
            });
            cur_path_parent
        })
    }
}
