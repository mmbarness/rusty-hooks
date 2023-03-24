use std::{sync::Arc, path::PathBuf};
use notify::Event;
use crate::runner::types::SpawnMessage;

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

pub type SubscribeReceiver = BroadcastReceiver<SpawnMessage>;
pub type SubscribeSender = BroadcastSender<SpawnMessage>;
pub type SubscribeChannel = Channel<SpawnMessage>;
pub type UnsubscribeChannel = Channel<PathBuf>;