use std::path::PathBuf;
use crate::watcher::watcher_scripts::Script;

pub type SpawnMessage = (PathBuf, Vec<Script>);
// pub type SpawnChannel = 