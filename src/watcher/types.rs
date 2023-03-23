use std::{sync::{Arc, MutexGuard}, path::PathBuf, collections::HashMap};
use chrono::{DateTime, Utc};
use notify::Event;
use tokio::sync::Mutex;
use crate::runner::types::SubscribeMessage;

use u64 as path_hash;

pub type BroadcastSender<T> = tokio::sync::broadcast::Sender<T>;
pub type BroadcastReceiver<T> = tokio::sync::broadcast::Receiver<T>;
pub type Channel<T> = (
    BroadcastSender<T>,
    BroadcastReceiver<T>
);

pub type EventMessage = Result<Event, Arc<notify::Error>>;
pub type EventsReceiver = BroadcastReceiver<EventMessage>;
pub type EventsSender = BroadcastSender<EventMessage>;
pub type EventChannel = Channel<EventMessage>;

pub type SubscribeReceiver = BroadcastReceiver<SubscribeMessage>;
pub type SubscribeSender = BroadcastSender<SubscribeMessage>;
pub type SubscribeChannel = Channel<SubscribeMessage>;
pub type UnsubscribeChannel = Channel<PathBuf>;

pub type PathHash = path_hash;
pub type PathsCache<'a> = MutexGuard<'a, HashMap<PathHash, SubscribeMessage>>;

pub type TimerController = Arc<Mutex<(chrono::Duration,DateTime<Utc>)>>;