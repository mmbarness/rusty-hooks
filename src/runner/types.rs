use std::path::PathBuf;
use crate::scripts::r#struct::Script;

pub type SpawnMessage = (PathBuf, Vec<Script>);
// pub type SpawnChannel = 