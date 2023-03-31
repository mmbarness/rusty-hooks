use std::{collections::HashMap, path::{Path, PathBuf}};
use notify::EventKind;
use serde::{Deserialize, Serialize};
use crate::utilities::traits::Utilities;

#[cfg(test)]
use mocktopus::macros::*;

#[cfg_attr(test, mockable)]
#[derive(Debug, Clone)]
pub struct Scripts {
    pub scripts_by_event_triggers: ScriptsByEventTrigger
}

pub type ScriptsByEventTrigger = HashMap<EventKind, Vec<Script>>; // string identifies the event type, Vec<ScriptSchemas> are all scripts that should run on a given event

#[cfg_attr(test, mockable)]
impl Utilities for Scripts {}

#[cfg_attr(test, mockable)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptJSON {
    pub event_triggers: Vec<String>,
    pub file_name: String,
    pub watch_path: String,
    pub run_delay: u8,
}

#[cfg_attr(test, mockable)]
#[derive(Debug, Clone)]
pub struct Script {
    pub event_triggers: Vec<String>,
    pub file_path: PathBuf,
    pub file_name: String,
    pub failed: Option<bool>,
    pub run_delay: u8,
    pub watch_path: PathBuf,
}

#[cfg_attr(test, mockable)]
impl Utilities for Script {}

#[cfg_attr(test, mockable)]
impl From<ScriptJSON> for Script {
    fn from(json: ScriptJSON) -> Self {
        let path_string = format!("./user_scripts/{}", json.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&json.watch_path).to_path_buf();

        Script {
            event_triggers: json.event_triggers,
            file_path: as_path,
            file_name: json.file_name,
            failed: None,
            run_delay: json.run_delay,
            watch_path
        }
    }
}

#[cfg_attr(test, mockable)]
impl From<&ScriptJSON> for Script {
    fn from(json: &ScriptJSON) -> Self {
        let path_string = format!("./user_scripts/{}", json.file_name.clone());
        let as_path = Path::new(&path_string).to_path_buf();

        let watch_path = Path::new(&json.watch_path).to_path_buf();
        Script {
            event_triggers: json.event_triggers.clone(),
            file_path: as_path,
            file_name: json.file_name.clone(),
            failed: None,
            run_delay: json.run_delay,
            watch_path
        }
    }
}
