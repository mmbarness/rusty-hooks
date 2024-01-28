use log::{error, debug, info};
use tokio::{sync::broadcast::{error::RecvError, Sender}, task::JoinHandle};
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
            events_emitter,
            events_receiver
        ) = tokio::sync::broadcast::channel::<Result<Event, Arc<notify::Error>>>(16);
        let events_channel_clone = events_emitter.clone();

        let notify_watcher = RecommendedWatcher::new(move |res| {
            match res {
                Ok(event) => {
                    match events_emitter.send(Ok(event)) {
                        Ok(_) => {},
                        Err(e) => {
                            error!("error sending new event: {:?}", e)
                        }
                    };
                }
                Err(e) => {
                    let error_arc = Arc::new(e);
                    events_emitter.send(Err(error_arc)).unwrap();
                }
            }
        }, Config::default())?;

        Ok((notify_watcher, (events_channel_clone, events_receiver)))
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
                events_emitter,
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
            events_emitter,
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
        debug!("unwatching path, tasks will close");
        notifier_handle.unwatch(watch_path.as_ref()).map_err(EventError::NotifyError)?;

        Ok(())
    }

    /// Uses the watcher runtime to spawn distinct threads for 1) receiving events delivered by [`notify`] containing
    /// information about new events occurring at paths Watcher is meant to observe, 2) if the event contains a path
    /// that should be subscribed to, creating that subscription, and 3) managing unsubscribing from paths already
    /// processed by the relevant scripts.
    fn initialize_watcher_tasks(
        &self,
        events_emitter:Sender<Result<Event, Arc<notify::Error>>>,
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
        JoinHandle<Result<(), RecvError>>
    ) {
        let subscriber_channel_1 = path_subscriber.subscribe_channel.0.clone();
        let subscriber_channel_2 = path_subscriber.subscribe_channel.0.clone();
        let paths_clone = path_subscriber.paths.clone();
        let unsubscribe_receiver = unsubscribe_channel.subscribe();
        let watch_path_clone = watch_path.clone();

        // start watching for new events from the notify crate
        let events_task: JoinHandle<Result<(), RecvError>> = self.runtime.spawn(async move {
            Self::watch_events(
                events_receiver,
                watch_path_clone,
                scripts,
                subscriber_channel_2
            ).await
        });

        // start watching for new path subscriptions coming from the event watcher
        let subscription_task = self.runtime.spawn(async move {
            path_subscriber.route_subscriptions(
                events_emitter.clone(),
                spawn_channel,
                subscriber_channel_1,
                paths_clone
            ).await
        });

        // start watching for paths to *unsubscribe* from
        let unsubscribe_task = self.runtime.spawn(async move {
            PathSubscriber::unsubscribe_task(unsubscribe_receiver, paths, watch_path).await
        });

        (unsubscribe_task, subscription_task, events_task)
    }

    /// Uses [`tokio::select!`] to kill *all* futures if *any* fail. If the event loop exits,
    /// for example, the task watching for new path subscriptions will too.
    async fn handle_all_futures(events: JoinHandle<Result<(), RecvError>>, subscriptions:JoinHandle<Result<(), SubscriptionError>>, unsubscription: JoinHandle<Result<(), SubscriptionError>>) -> () {
        tokio::select! {
            a = events => {
                match a {
                    Ok(a) => {
                        debug!("Events_task exited. Error: {:?}. Historically this has been because of the events channel closing.", a.err());
                    },
                    Err(e) => {
                        debug!("events_task failed: {}, exiting", e);
                    },
                }
            },
            b = subscriptions => {
                match b {
                    Ok(b) => {
                        debug!("Subscription_task exited. Error: {:?}", b.err());
                    },
                    Err(e) => {
                        debug!("subscription_task failed: {}, exiting", e);
                    }
                }
            },
            c = unsubscription => {
                match c {
                    Ok(c) => {
                        debug!("unsubscribe_task exited: {:?}", c.err());
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
