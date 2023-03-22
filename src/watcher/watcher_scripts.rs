use std::{process::{Child, Command}, fs::{self}, thread::{self, JoinHandle}, sync::{ Arc }, collections::HashMap};
use log::{info, debug};
use notify::{EventKind, event::{AccessKind}, Event};
use serde::{Deserialize, Serialize};
use super::watcher_errors::{script_error::ScriptError, spawn_error::SpawnError};


#[derive(Debug, Clone)]
pub struct WatcherScripts {
    pub scripts_by_event_triggers: ScriptsByEventTrigger,
}

pub type Threads = Option<Vec<JoinHandle<Result<Child, ScriptError>>>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Script {
    pub event_triggers: Vec<String>,
    pub file_name: String,
    pub failed: Option<bool>,
    pub run_delay: u8,
}

type ScriptsByEventTrigger = HashMap<EventKind, Vec<Script>>; // string identifies the event type, Vec<ScriptSchemas> are all scripts that should run on a given event

impl WatcherScripts {
    pub fn ingest_configs(configs_path: &String) -> Result<Self, ScriptError> {
        let configs_file = fs::read_to_string(format!("{}/scripts_config.json", configs_path))?;

        let files = serde_json::from_str::<Vec<Script>>(&configs_file)?;

        for file in &files {
            let message = serde_json::to_string_pretty(&file);
            info!("script configuration: {:?}", message);
        }   

        match files.clone().into_iter().fold(true, |valid_so_far, current| {
            if !valid_so_far {
                return valid_so_far
            }
            let path = format!("./scripts/{}", current.file_name.clone());
            info!("path of identified script: {}", path);
            is_executable::is_executable(path)
        }) {
            true => {},
            false => return Err(ScriptError::GenericMessage("unable to validate scripts folder".into())),
        }
        let scripts_by_event_triggers = Self::cache_scripts_by_events(&files);
        debug!("all scripts by event type: {:?}", &scripts_by_event_triggers);
        Ok(WatcherScripts{
            scripts_by_event_triggers,
        })
    }

    fn cache_scripts_by_events(files: &Vec<Script>) -> HashMap<EventKind, Vec<Script>> {
        let acc_int:ScriptsByEventTrigger = HashMap::new();
        let files_iter = files.clone().into_iter();
        // iterate over each file and reduce to hashmap of event type and associated scripts to run
        files_iter.fold(acc_int, |scripts_by_event_type_acc, current| {
            let current_event_triggers = &current.event_triggers;
            // need to reduce over the array of events a given script schema should be tied to and update the upper-level accumulator accordingly
            current_event_triggers.into_iter().fold(scripts_by_event_type_acc, |mut scripts_by_event_type_acc, event| {
                let event_string = event.clone();
                let event_kind = match event_string.as_str() {
                    "Access" => EventKind::Access(AccessKind::Any),
                    "Create" => EventKind::Create(notify::event::CreateKind::Any),
                    "Modify" => EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::To)),
                    "Remove" => EventKind::Remove(notify::event::RemoveKind::Any),
                    "Other" => EventKind::Other,
                    _ => {
                        return scripts_by_event_type_acc
                    }
                };
                let event_schemas = Self::update_schema_vec(&event_kind, current.clone(), &mut scripts_by_event_type_acc);
                scripts_by_event_type_acc.insert(event_kind.clone(), event_schemas);
                scripts_by_event_type_acc
            })
        })
    }

    fn update_schema_vec(event_type: &EventKind, new_script: Script, scripts: &mut HashMap<EventKind, Vec<Script>>) -> Vec<Script> {
        debug!("deciding whether to insert script {:?}", serde_json::to_string_pretty(&new_script.file_name));
        match scripts.get_mut(event_type) {
            Some(acc_event_type_scripts) => { // array of scripts attached to event_type
                let script_exists = acc_event_type_scripts.into_iter().any(|script| {
                    new_script.file_name.eq_ignore_ascii_case(&script.file_name)
                });
                match script_exists {
                    true => {
                        acc_event_type_scripts.to_vec()
                    }
                    false => {
                        acc_event_type_scripts.push(new_script.clone());
                        acc_event_type_scripts.to_vec()
                    }
                }
            },
            None => {
                let event_type_and_schema_to_insert = vec![new_script.clone()];
                scripts.insert(event_type.clone(), vec![new_script.clone()]);
                // accumulator_copy
                event_type_and_schema_to_insert
            }
        }
    }

    pub fn run_event(&self, event: Event) -> Result<Option<Vec<JoinHandle<Result<Child, ScriptError>>>>, ScriptError> {
        info!("attempting to run event of type: {:?}", serde_json::to_string(&event.kind));

        let event_scripts = match self.scripts_by_event_triggers.get(&event.kind) {
            Some(scripts) => scripts.clone(),
            None => return Ok(None)
        };

        let threads:Vec<JoinHandle<Result<Child, ScriptError>>> = event_scripts.iter().clone().map(move |script| {
            let file_arc = Arc::new(script.clone());
            thread::spawn(move || {
                let path = format!("./scripts/{}", file_arc.file_name);
                let process = match Self::run(&path) {
                    Ok(child) => Ok(child),
                    Err(e) => Err(e)
                };
                process
            })
        }).collect();

        Ok(Some(threads))
    }
}

pub trait SpawnWatchScript {
    fn run(path: &String) -> Result<Child, ScriptError> {
        info!("attempting to run script at: {}", path.clone().to_string());
        Ok(Command::new("sh")
            .arg("-C")
            .arg(path.to_string())
            .spawn()?)
    }

    fn validate_scripts(path: &String) -> Result<bool, SpawnError> {
        let paths = fs::read_dir(path.clone())?;

        let all_executable = paths.fold(true, |valid_so_far, path|  {
            if !valid_so_far {
                return valid_so_far
            }
            let current_valid = match path {
                Ok(p) => p,
                Err(_) => return false,
            };
            is_executable::is_executable(current_valid.path())
        });

        Ok(all_executable)
    }

}

impl SpawnWatchScript for WatcherScripts {}
