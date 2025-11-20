use super::types::PathHash;
use crate::scripts::structs::Script;
use crate::utilities::{
    thread_types::{SubscribeChannel, UnsubscribeChannel},
    traits::Utilities,
};
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

/// Watches for events at a given path and executes scripts hooked to that path when appropriate.
#[derive(Debug)]
pub struct Watcher {}

#[derive(Debug)]
pub struct PathSubscriber {
    /// Concurrently updated data structure containing all paths currently subscribed to.
    pub paths: Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>,
    /// MPSC channel over which new path subscriptions are sent.
    pub subscribe_channel: SubscribeChannel,
    /// MPSC channel by which paths are unsubscribed from.
    pub unsubscribe_channel: UnsubscribeChannel,
}

impl Utilities for PathSubscriber {}

impl Utilities for Watcher {}
