use tokio::{sync::Mutex, runtime::Runtime};
use std::sync::Arc;
use std::{path::PathBuf, collections::HashMap};
use super::types::PathHash;
use crate::scripts::r#struct::Script;
use crate::utilities::{thread_types::{SubscribeChannel, UnsubscribeChannel}, traits::Utilities};

#[derive(Debug)]
pub struct Watcher {
    pub runtime: Arc<Mutex<Runtime>>,
    pub subscriber: Arc<Mutex<PathSubscriber>>,
}

#[derive(Debug)]
pub struct PathSubscriber {
    pub paths: Arc<std::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>,
    pub subscribe_channel: SubscribeChannel,
    pub unsubscribe_channel: UnsubscribeChannel
}

impl Utilities for PathSubscriber {}

impl Utilities for Watcher {}