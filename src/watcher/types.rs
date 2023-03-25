use std::{sync::{MutexGuard, Arc}, collections::HashMap, path::PathBuf};
use crate::{runner::types::SpawnMessage, scripts::structs::Script};
use u64 as path_hash;

pub type PathHash = path_hash;
pub type PathsCache<'a> = MutexGuard<'a, HashMap<PathHash, SpawnMessage>>;
pub type PathsCacheArc = Arc<std::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>;