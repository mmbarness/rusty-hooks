use std::path::PathBuf;
use crate::scripts::structs::Script;

pub type SpawnMessage = (PathBuf, Vec<Script>);
// pub type SpawnChannel = 