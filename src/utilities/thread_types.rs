use crate::runner::types::SpawnMessage;
use notify::Event;
use std::{path::PathBuf, sync::Arc};

pub type BroadcastSender<T> = tokio::sync::broadcast::Sender<T>;
pub type BroadcastReceiver<T> = tokio::sync::broadcast::Receiver<T>;
pub type Channel<T> = (BroadcastSender<T>, BroadcastReceiver<T>);

pub type EventMessage = Result<Event, Arc<notify::Error>>;
pub type EventsReceiver = BroadcastReceiver<EventMessage>;
pub type EventChannel = Channel<EventMessage>;

pub type SubscribeSender = BroadcastSender<SpawnMessage>;
pub type SpawnSender = BroadcastSender<SpawnMessage>;
pub type UnsubscribeSender = BroadcastSender<PathBuf>;
pub type SubscribeChannel = Channel<SpawnMessage>;
pub type UnsubscribeChannel = Channel<PathBuf>;

