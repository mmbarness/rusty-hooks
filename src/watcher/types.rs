use std::{sync::MutexGuard, collections::HashMap};
use crate::runner::types::SpawnMessage;
use u64 as path_hash;

pub type PathHash = path_hash;
pub type PathsCache<'a> = MutexGuard<'a, HashMap<PathHash, SpawnMessage>>;
