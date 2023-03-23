use tokio::{sync::{Mutex}, runtime::{Runtime}};
use std::{sync::Arc};
use crate::utilities::r#trait::Utilities;

use super::{path_subscriber::PathSubscriber};

#[derive(Debug)]
pub struct Watcher {
    pub runtime: Arc<Mutex<Runtime>>,
    pub subscriber: Arc<Mutex<PathSubscriber>>,
}

impl Utilities for Watcher {}