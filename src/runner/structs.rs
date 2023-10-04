use std::path::PathBuf;
use tokio::sync::broadcast::{Receiver, Sender};
use crate::utilities::traits::Utilities;
use crate::scripts::structs::Script;
#[cfg(test)]
use mocktopus::macros::*;

#[cfg_attr(test, mockable)]
#[derive(Debug)]
pub struct Runner {
    pub runtime: tokio::runtime::Runtime,
    pub spawn_channel: (Sender<(PathBuf, Vec<Script>)>, Receiver<(PathBuf, Vec<Script>)>),
    pub unsubscribe_broadcast_channel: (Sender<PathBuf>, Receiver<PathBuf>)
}

#[cfg_attr(test, mockable)]
impl Utilities for Runner {}
