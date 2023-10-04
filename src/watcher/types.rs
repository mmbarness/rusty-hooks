use std::{sync::Arc, collections::HashMap, path::PathBuf};
use crate::{runner::types::SpawnMessage, scripts::structs::Script};
use tokio::sync::MutexGuard;
use u64 as path_hash;

pub type PathHash = path_hash;
pub type PathsCache<'a> = MutexGuard<'a, HashMap<PathHash, SpawnMessage>>;
pub type PathsCacheArc = Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>;
