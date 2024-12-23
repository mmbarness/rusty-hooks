use crate::utilities::traits::Utilities;
use notify::EventKind;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Scripts {
    pub scripts_by_event_triggers: ScriptsByEventTrigger,
    pub watch_paths: Vec<PathBuf>,
}

pub type ScriptsByEventTrigger = HashMap<EventKind, Vec<Script>>; // string identifies the event type, Vec<ScriptSchemas> are all scripts that should run on a given event

impl Utilities for Scripts {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptJSON {
    pub enabled: bool,
    pub event_triggers: Vec<String>,
    pub file_name: String,
    pub watch_path: String,
    pub run_delay: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptBlock {
    pub name: String,
    pub description: String,
    pub file_name: String,
    pub watch_path: String,
    pub enabled: bool,
    pub run_delay: u8,
    pub event_triggers: Vec<String>,
    pub dependencies: Vec<Option<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptYAML {
    pub scripts: Vec<ScriptBlock>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Script {
    pub event_triggers: Vec<String>,
    pub file_path: PathBuf,
    pub file_name: String,
    pub failed: Option<bool>,
    pub run_delay: u8,
    pub watch_path: PathBuf,
}

impl Utilities for Script {}

impl From<ScriptJSON> for Script {
    fn from(json: ScriptJSON) -> Self {
        let path_string = format!("./scripts/{}", json.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&json.watch_path).to_path_buf();

        Script {
            event_triggers: json.event_triggers,
            file_path: as_path,
            file_name: json.file_name,
            failed: None,
            run_delay: json.run_delay,
            watch_path,
        }
    }
}

impl From<ScriptBlock> for Script {
    fn from(block: ScriptBlock) -> Self {
        let path_string = format!("./scripts/{}", block.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&block.watch_path).to_path_buf();

        Script {
            event_triggers: block.event_triggers,
            file_path: as_path,
            file_name: block.file_name,
            failed: None,
            run_delay: block.run_delay,
            watch_path,
        }
    }
}

impl From<&ScriptBlock> for Script {
    fn from(yaml: &ScriptBlock) -> Self {
        let path_string = format!("./scripts/{}", yaml.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&yaml.watch_path).to_path_buf();

        Script {
            event_triggers: yaml.event_triggers.clone(),
            file_path: as_path,
            file_name: yaml.file_name.clone(),
            failed: None,
            run_delay: yaml.run_delay,
            watch_path,
        }
    }
}

impl From<&ScriptJSON> for Script {
    fn from(json: &ScriptJSON) -> Self {
        let path_string = format!("./scripts/{}", json.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&json.watch_path).to_path_buf();
        Script {
            event_triggers: json.event_triggers.clone(),
            file_path: as_path,
            file_name: json.file_name.clone(),
            failed: None,
            run_delay: json.run_delay,
            watch_path,
        }
    }
}
