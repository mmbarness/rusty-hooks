use std::path::PathBuf;
use crate::watcher::watcher_scripts::Script;

pub type SubscribeMessage = (PathBuf, Vec<Script>);