use tokio::runtime::Runtime;
use std::sync::Arc;
use std::{path::PathBuf, collections::HashMap};
use super::types::PathHash;
use crate::scripts::structs::Script;
use crate::utilities::{thread_types::{SubscribeChannel, UnsubscribeChannel}, traits::Utilities};

/// Watches for events at a given path and executes scripts hooked to that path when appropriate.
#[derive(Debug)]
pub struct Watcher {
    /// Runtime used to spawn the tasks need to watch for new subscriptions, unsubscriptions, and events.
    pub runtime: Runtime,
}

#[derive(Debug)]
pub struct PathSubscriber {
    /// Concurrently updated data structure containing all paths currently subscribed to.
    pub paths: Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>,
    /// MPSC channel over which new path subscriptions are sent.
    pub subscribe_channel: SubscribeChannel,
    /// MPSC channel by which paths are unsubscribed from.
    pub unsubscribe_channel: UnsubscribeChannel,
    /// Runtime used to run timers in parallel.
    pub wait_threads: Runtime
}

impl Utilities for PathSubscriber {}

impl Utilities for Watcher {}
