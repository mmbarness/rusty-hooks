use log::{debug, info};
use tokio::{sync::{broadcast::Sender, TryLockError}, task::JoinHandle};
use std::{sync::Arc, path::PathBuf};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};
use crate::{utilities::thread_types::UnsubscribeSender, errors::watcher_errors::{event_error::EventError, subscriber_error::SubscriptionError}};
use crate::scripts::structs::{Scripts, Script};
use crate::errors::watcher_errors::watcher_error::WatcherError;
use crate::utilities::{traits::Utilities, thread_types::EventChannel};
use super::structs::{PathSubscriber, Watcher};

impl Watcher {
    pub fn new() -> Result<Self, WatcherError> {
        let watcher_runtime = <Self as Utilities>::new_runtime(4, &"watcher-runtime".to_string())?;
        Ok(Watcher {
            runtime: watcher_runtime,
        })
    }

    /// Creates a new [`notify::RecommendedWatcher`] and passes the RecommendedWatcher a callback to emit events over an MPSC channel
    /// monitored in the route_subscriptions function.
    fn notifier_task() -> notify::Result<(
        RecommendedWatcher, EventChannel
    )> {
        let (
            emit_events_channel,
            receive_events_channel
        ) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let events_channel_clone = emit_events_channel.clone();

        let notify_watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(r) => {
                    let _ = emit_events_channel.send(Ok(r));
                }
                Err(e) => {
                    let error_arc = Arc::new(e);
                    emit_events_channel.send(Err(error_arc)).unwrap();
                }
            }
        }, Config::default())?;

        Ok((notify_watcher, (events_channel_clone, receive_events_channel)))
    }

    /// Begins watching a certain path. Uses its runtime to initialize threads to accept new subscriptions on, accept unsubscriptions on,
    /// and to watch for new events.
    pub async fn start(
        &self, spawn_channel: Sender<(PathBuf, Vec<Script>)>, unsubscribe_channel: UnsubscribeSender, watch_path: PathBuf, scripts: &Scripts
    ) -> Result<(), WatcherError> {
        info!("now watching path: {}", &watch_path.to_str().unwrap());
        let (
            mut notifier_handle,
            (
                emit_events_channel,
                events_receiver
            )
        ) = Self::notifier_task().map_err(EventError::NotifyError)?;

        notifier_handle.watch(watch_path.as_ref(), RecursiveMode::Recursive).map_err(EventError::NotifyError)?;

        let path_subscriber = PathSubscriber::new()?;

        let (
            unsubscribe_task,
            subscribe_task,
            events_task
        ) = Self::initialize_watcher_tasks(
            &self,
            emit_events_channel,
            events_receiver,
            path_subscriber.paths.clone(),
            path_subscriber,
            scripts.clone(),
            spawn_channel,
            unsubscribe_channel,
            watch_path.clone(),
        );

        Self::handle_all_futures(events_task, subscribe_task, unsubscribe_task).await;

        // cleanup
        notifier_handle.unwatch(watch_path.as_ref()).map_err(EventError::NotifyError)?;

        Ok(())
    }

    fn initialize_watcher_tasks(
        &self,
        emit_events_channel:Sender<Result<Event, Arc<notify::Error>>>,
        events_receiver: tokio::sync::broadcast::Receiver<Result<Event, Arc<notify::Error>>>,
        paths: Arc<tokio::sync::Mutex<std::collections::HashMap<u64, (PathBuf, Vec<Script>)>>>,
        path_subscriber: PathSubscriber,
        scripts: Scripts,
        spawn_channel: Sender<(PathBuf, Vec<Script>)>,
        unsubscribe_channel: UnsubscribeSender,
        watch_path: PathBuf,
    ) -> (
        JoinHandle<Result<(), SubscriptionError>>,
        JoinHandle<Result<(), SubscriptionError>>,
        JoinHandle<Result<(), TryLockError>>
    ) {
        let subscriber_channel_1 = path_subscriber.subscribe_channel.0.clone();
        let subscriber_channel_2 = path_subscriber.subscribe_channel.0.clone();
        let paths_clone = path_subscriber.paths.clone();
        let unsubscribe_receiver = unsubscribe_channel.subscribe();
        let watch_path_clone = watch_path.clone();

        let event_channel_for_path_subscriber = emit_events_channel.clone();
        // start watching for paths to *unsubscribe* from
        let unsubscribe_task = self.runtime.spawn(async move {
            PathSubscriber::unsubscribe_task(unsubscribe_receiver, paths, watch_path).await
        });
        // start watching for new path subscriptions coming from the event watcher
        let subscription_task = self.runtime.spawn(async move {
            path_subscriber.route_subscriptions(
                event_channel_for_path_subscriber,
                spawn_channel,
                subscriber_channel_1,
                paths_clone
            ).await
        });

        // start watching for new events from the notify crate
        let events_task = self.runtime.spawn(async move {
            Self::watch_events(
                events_receiver,
                watch_path_clone,
                scripts,
                subscriber_channel_2
            ).await
        });

        (unsubscribe_task, subscription_task, events_task)
    }

    /// Uses [`tokio::select!`] to kill *all* futures if *any* fail. If the event loop exits,
    /// for example, the task watching for new path subscriptions will too.
    async fn handle_all_futures(events: JoinHandle<Result<(), TryLockError>>, subscriptions:JoinHandle<Result<(), SubscriptionError>>, unsubscription: JoinHandle<Result<(), SubscriptionError>>) -> () {
        debug!("now awaiting events, subscriptions, and unsubscriptions task");
        tokio::select! {
            a = events => {
                debug!("events_task exited");
                match a {
                    Ok(_) => {
                        debug!("events_task exited");
                    },
                    Err(e) => {
                        debug!("events_task failed: {}, exiting", e);
                    }
                }
            },
            b = subscriptions => {
                debug!("subscription_task exited");
                match b {
                    Ok(_) => {
                        debug!("subscription_task exited");
                    },
                    Err(e) => {
                        debug!("subscription_task failed: {}, exiting", e);
                    }
                }
            },
            c = unsubscription => {
                debug!("unsubscribe_task exited");
                match c {
                    Ok(_) => {
                        debug!("unsubscribe_task exited");
                    },
                    Err(e) => {
                        debug!("unsubscription_task failed: {}, exiting", e);
                    }
                }
            }
        }
        info!("when one task exits for whatever reason (probably an error), all are killed and the program exits")
    }
}
