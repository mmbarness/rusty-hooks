use std::{process::{Child}, fs::{self}, str::FromStr, thread::{self, JoinHandle}, sync::{ Arc }, collections::HashMap};
use log::{info, debug};
use serde::{Deserialize, Serialize};
use crate::syncthing::errors::SpawnError;
use super::{errors::{ScriptsError}, event_structs::EventTypes};
use crate::syncthing::spawn_script::Spawn;

#[derive(Debug)]
pub struct Scripts {
    scripts_by_event_triggers: ScriptsByEventTrigger,
    threads: Threads,
}

pub type Threads = Option<Vec<JoinHandle<Result<Child, ScriptsError>>>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptSchema {
    event_triggers: Vec<String>,
    file_name: String,
    failed: Option<bool>
}

type EventString = String;
type ScriptsByEventTrigger = HashMap<EventString, Vec<ScriptSchema>>; // string identifies the event type, Vec<ScriptSchemas> are all scripts that should run on a given event

impl Scripts {
    pub fn ingest_configs(configs_path: &String) -> Result<Self, ScriptsError> {
        let configs_file = fs::read_to_string(format!("{}/scripts_config.json", configs_path))?;

        let files = serde_json::from_str::<Vec<ScriptSchema>>(&configs_file)?;

        match files.clone().into_iter().fold(true, |valid_so_far, current| {
            if !valid_so_far {
                return valid_so_far
            }
            let path = format!("./scripts/{}", current.file_name.clone());
            info!("path of identified script: {}", path);
            is_executable::is_executable(path)
        }) {
            true => {},
            false => return Err(ScriptsError::GenericMessage("unable to validate scripts folder".to_string())),
        }
        let acc_int:ScriptsByEventTrigger = HashMap::new();
        let scripts_by_event_triggers = files.clone().into_iter().fold(acc_int, |scripts_by_event_type_acc, current| {
            let current_event_triggers = &current.event_triggers;
            // need to reduce over the array of events a given script schema should be tied to and update the upper-level accumulator accordingly
            let updated_acc = current_event_triggers.into_iter().fold(scripts_by_event_type_acc, |mut scripts_by_event_type_acc, event| {
                let event_type = event.clone();
                let event_schemas = Self::update_schema_vec(&event_type, current.clone(), &mut scripts_by_event_type_acc);
                scripts_by_event_type_acc.insert(event_type.clone(), event_schemas);
                scripts_by_event_type_acc
            });
            updated_acc
        });
        debug!("all scripts by event type: {:?}", &scripts_by_event_triggers);
        Ok(Scripts{
            scripts_by_event_triggers,
            threads: None
        })
    }

    fn update_schema_vec(event_type: &String, new_schema: ScriptSchema, scripts: &mut HashMap<String, Vec<ScriptSchema>>) -> Vec<ScriptSchema> {
        debug!("deciding whether to insert script {:?}", serde_json::to_string_pretty(&new_schema.file_name));
        match scripts.get_mut(event_type) {
            Some(acc_event_type_scripts) => {
                let addition_needed = match acc_event_type_scripts.into_iter().find(|script| {
                    match &new_schema.file_name.eq_ignore_ascii_case(&script.file_name) {
                        true => {
                            debug!("new schema file name of {:?} matches script of file name {:?}", &new_schema.file_name, &script.file_name);
                            false
                        },
                        false => {
                            debug!("new schema file name of {:?} does not match script of file name {:?}", &new_schema.file_name, &script.file_name);
                            true
                        }
                    }
                }) {
                    Some(_) => {
                        debug!("setting addition_needed to {}", true);
                        true
                    },
                    None => {
                        debug!("setting addition_needed to {}", false);
                        false
                    },
                };
                match addition_needed {
                    true => {
                        acc_event_type_scripts.push(new_schema.clone());
                        acc_event_type_scripts.clone()
                    }
                    false => {
                        acc_event_type_scripts.clone()
                    }
                }
            },
            None => {
                debug!("no pre-existing key of {} found in accumulator", event_type);
                // insert new_schema into accumulator
                let mut accumulator_copy= scripts.clone();
                let event_type_and_schema_to_insert = vec![new_schema.clone()];
                accumulator_copy.insert(event_type.clone(), event_type_and_schema_to_insert.clone());
                // accumulator_copy
                event_type_and_schema_to_insert
            }
        }
    }

    pub fn run_event(&self, event_type: String) -> Result<Option<Vec<JoinHandle<Result<Child, ScriptsError>>>>, ScriptsError> {
        info!("attempting to run event of type: {}", event_type);
        let validated_event = match EventTypes::from_str(&event_type) {
            Ok(event) => event,
            Err(e) => {
                return Err(SpawnError::ParseError(e).into())
            }
        };

        let event_scripts = match self.scripts_by_event_triggers.get(validated_event.as_ref()) {
            Some(scripts) => scripts.clone(),
            None => return Ok(None)
        };

        info!("found {} events to run for type {}", event_scripts.len(), event_type);

        let threads:Vec<JoinHandle<Result<Child, ScriptsError>>> = event_scripts.iter().clone().map(move |script| {
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