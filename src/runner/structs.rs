use std::{path::PathBuf, sync::{Mutex, Arc}};
use tokio::sync::broadcast::{Receiver, Sender};
use crate::utilities::traits::Utilities;
use crate::scripts::structs::Script;

pub struct Runner {
    pub runtime: Arc<Mutex<tokio::runtime::Runtime>>,
    pub spawn_channel: (Sender<(PathBuf, Vec<Script>)>, Receiver<(PathBuf, Vec<Script>)>),
    pub unsubscribe_broadcast_channel: (Sender<PathBuf>, Receiver<PathBuf>)
}

impl Utilities for Runner {}