use crate::{runner::types::SpawnMessage, scripts::structs::Script};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::MutexGuard;
use u64 as path_hash;

pub type PathHash = path_hash;
pub type PathsCache<'a> = MutexGuard<'a, HashMap<PathHash, SpawnMessage>>;
pub type PathsCacheArc = Arc<tokio::sync::Mutex<HashMap<PathHash, (PathBuf, Vec<Script>)>>>;
