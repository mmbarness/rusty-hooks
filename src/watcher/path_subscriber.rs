use std::{path::PathBuf, collections::HashMap, sync::Arc};
use log::{debug, error, info};
use notify::Event;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::runtime::{Handle, Runtime};
use crate::errors::runtime_error::enums::RuntimeError;
use crate::errors::watcher_errors::event_error::EventError;
use crate::errors::watcher_errors::subscriber_error::SubscriptionError;
use crate::errors::watcher_errors::thread_error::UnexpectedAnyhowError;
use crate::scripts::structs::Script;
use crate::runner::types::SpawnMessage;
use crate::utilities::{thread_types::{BroadcastReceiver, EventMessage, BroadcastSender}, traits::Utilities};
use super::structs::PathSubscriber;
use crate::errors::watcher_errors::thread_error::ThreadError;
use super::types::{PathHash, PathsCache, PathsCacheArc};

impl PathSubscriber {
    pub fn new() -> Result<Self, SubscriptionError> {
        let path_cache:HashMap<PathHash, (PathBuf, Vec<Script>)> = HashMap::new();
        let paths = Arc::new(tokio::sync::Mutex::new(path_cache));
        let wait_threads = Self::new_runtime(4, &"timer-threads".to_string())?;
        Ok(PathSubscriber {
            paths,
            subscribe_channel: Self::new_channel::<SpawnMessage>(),
            unsubscribe_channel: Self::new_channel::<PathBuf>(),
            wait_threads,
        })
    }

    pub fn unsubscribe(path: &PathBuf, mut paths: PathsCache<'_>) -> Result<(), SubscriptionError> {
        let path_string = path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());
        match paths.remove_entry(&path_hash) {
            Some(_) => Ok(()),
            None => Err(SubscriptionError::UnsubscribeError(format!("didn't find path in cache, didnt unsubscribe")))
        }
    }

    pub async fn unsubscribe_task(mut unsubscribe_channel: Receiver<PathBuf>, paths: PathsCacheArc, watch_path: PathBuf) -> Result<(), SubscriptionError> {
        debug!("spawned unsubscribe thread");
        loop {
            let path = unsubscribe_channel.recv().await.map_err(ThreadError::RecvError)?;
            let path_str = path.to_str().unwrap_or("failed path string parse");
            let watch_path_string = watch_path.to_str().unwrap_or("failed watch path parse");
            if !Self::path_contains_subdir(&watch_path, &path) {
                // single runner thread sends unsub messages across possible n watchers. need to filter out irrelevant unsub messages
                debug!("path {} NOT contained within watch path {}. skipping unsubscribe", path_str, watch_path_string);
                continue;
            } else {
                debug!("path {} contained within watch path {}, unsubscribing", path_str, watch_path_string)
            }
            let paths = match paths.try_lock() {
                Ok(p) => p,
                Err(e) => {
                    error!("unable to lock onto paths while trying to unsubscribe: {:?}", e);
                    continue;
                }
            };
            match PathSubscriber::unsubscribe(&path, paths) {
                Ok(_) => {
                    let path_display = path.display();
                    let unsubscribe_success_message = &format!("successfully unsubscribed from path: {}", path_display);
                    debug!("{}", unsubscribe_success_message)
                },
                Err(e) => {
                    error!("{}", e)
                }
            }
        }
    }

    pub fn validate_event_subscription(event:Result<Event, Arc<notify::Error>>, mut num_events_errors: i32) -> Result<(Event, i32), (SubscriptionError, i32)> {
        match event {
            Ok(event) => Ok((event, num_events_errors)),
            Err(e) => {
                let notify_error:SubscriptionError = match Arc::try_unwrap(e) {
                    Ok(error) => {
                        let event_error:EventError = error.into();
                        event_error.into()
                    },
                    Err(arc_error) => {
                        let unexpected_error = ThreadError::UnexpectedError(anyhow::Error::new(arc_error));
                        unexpected_error.into()
                    }
                };
                num_events_errors += 1;
                error!("error receiving events while waiting on timer to expire: {}", notify_error);
                Err((notify_error, num_events_errors))
            }
        }
    }

    async fn start_waiting(original_path: PathBuf, events_listener: BroadcastReceiver<EventMessage>) -> Result<(), SubscriptionError>{
        // thread that waits for events at particular path to end based on 1 or 2min timer and returns once either the events receiver closes or the timer runs out
        let new_timer = Self::new_timer(10);
        let timer_controller = new_timer.controller.clone();

        let handle = Handle::current();
        let timer_thread = handle.spawn(async move {
            println!("now using existing Runtime to wait out script timer");
            new_timer.wait().await
        });
        let events_thread = Self::event_loop(
            events_listener,
            original_path,
            timer_controller
        );
        match timer_thread.await.map_err(RuntimeError::JoinError)? {
            Ok(_) => {
                events_thread.abort();
                Ok(())
            },
            Err(e) => Err(e.into()),
        }
    }

    fn lock_and_update_paths(new_path:PathBuf, paths: Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>, scripts: Vec<Script>) -> Result<bool, SubscriptionError> {
        let mut paths_lock = paths.try_lock()?;

        let path_string = new_path.to_str().unwrap_or("unable to pull string out of path buf");
        let path_hash = Self::hasher(&path_string.to_string());

        // should only return true if the path isn't found in the datastructure
        let should_add_path = paths_lock.get(&path_hash).is_none();

        paths_lock.insert(path_hash, (new_path, scripts));

        Ok(should_add_path)
    }

    pub async fn route_subscriptions(
        self: &Self,
        events_emitter: BroadcastSender<EventMessage>,
        spawn_channel: BroadcastSender<SpawnMessage>,
        subscribe_channel: BroadcastSender<(PathBuf, Vec<Script>)>,
        paths: PathsCacheArc
    ) -> Result<(), SubscriptionError> {
        debug!("spawned subscribe thread");
        let mut subscription_listener = subscribe_channel.subscribe();
        let mut num_events_errors = 0;
        let mut last_error: Option<SubscriptionError> = None;
        loop {
            let (path, scripts) = match subscription_listener.recv().await.map_err(ThreadError::RecvError) {
                Ok(e) => e,
                Err(e) => {
                    num_events_errors += 1;
                    last_error = Some(e.into());
                    continue;
                }
            };

            let path_string = path.to_str().unwrap_or(&"bad path parse");
            debug!("new path: {}", path_string);

            if num_events_errors > 5 {
                let new_unexpected_error:ThreadError = ThreadError::new_unexpected_error(format!("error while waiting on events to run out at watched path {}", path_string));
                return Err(last_error.unwrap_or(new_unexpected_error.into()).into())
            };

            let subscribed_to_new_path = match Self::lock_and_update_paths(path.clone(), paths.clone(), scripts.clone()) {
                Ok(subscribed) => subscribed,
                Err(e) => {
                    error!("unable to subscribe to path: {}", e);
                    continue
                }
            };

            let path_str = path.to_str().unwrap_or("unable to read incoming path into string");
            match subscribed_to_new_path {
                true => {
                    debug!("watching new path at {}",path_str);
                },
                false => {
                    debug!("received new path subscription, but it's already being observed {}", path_str);
                }
            }
            if subscribed_to_new_path {
                let spawn_channel = spawn_channel.clone();
                // TODO: create channel for a timer and event thread to communicate, and spawn both on wait threads so that start_waiting need not spawn its own
                let _:JoinHandle<Result<(), SubscriptionError>> = Self::spawn_new_wait_thread(
                    events_emitter.subscribe(),
                    path,
                    &self.wait_threads,
                    scripts,
                    spawn_channel
                );
            }
        }
    }

    fn spawn_new_wait_thread(
        events: Receiver<Result<Event, Arc<notify::Error>>>,
        path: PathBuf,
        wait_threads: &Runtime,
        scripts: Vec<Script>,
        spawn_channel: tokio::sync::broadcast::Sender<(PathBuf, Vec<Script>)>
    ) -> JoinHandle<Result<(), SubscriptionError>> {
        wait_threads.spawn(async move {
            let wait_out_new_events_path = Self::start_waiting(path.clone(), events);
            wait_out_new_events_path.await?;
            info!("successfully waited on timer expiration, now running scripts");
            let stuff_to_send = (path.clone(), scripts);
            spawn_channel.send(stuff_to_send)?;
            Ok(())
        })
    }
}
