use std::{process::{Command, Child}, fs::{self, DirEntry, ReadDir}, str::FromStr, thread::{self, JoinHandle, Thread}, time::Duration, sync::{mpsc::channel, Mutex, Arc}, collections::HashMap, fmt};
use log::{info, debug};
use is_executable::IsExecutable;
use serde::Deserialize;
use strum::ParseError;
use threadpool::ThreadPool;
use super::{errors::SyncthingError, event_structs::EventTypes};

#[derive(Debug)]
pub struct Scripts {
    scripts_by_event_triggers: ScriptsByEventTrigger,
    threads: Option<Vec<JoinHandle<Result<Child, ScriptsError>>>>,
}

type EventString = String;
type ScriptId = String;
type ScriptsByEventTrigger = HashMap<EventString, Vec<ScriptSchema>>; // string identifies the event type, Vec<ScriptSchemas> are all scripts that should run on a given event

impl Scripts {
    pub fn ingest_configs() -> Result<Self, ScriptsError> {
        let configs_file = fs::read_to_string("./scripts/scripts_config.json")?;

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
        let scripts_by_event_triggers = files.clone().into_iter().fold(acc_int, |mut scripts_by_event_type_acc, current| {
            let current_file_path = &current.file_name;
            let mut current_event_triggers = &current.event_triggers;
            let event_schemas:ScriptsByEventTrigger = HashMap::new();
            let updated_acc = current_event_triggers.into_iter().fold(event_schemas, |mut _acc, event| {
                let event_type = event.clone();
                let event_schemas = match scripts_by_event_type_acc.get_mut(&event_type) {
                    Some(acc_event_type_scripts) => {
                        let addition_needed = match acc_event_type_scripts.into_iter().find(|script| {
                            match &current.file_name.eq_ignore_ascii_case(&script.file_name) {
                                true => false,
                                false => true
                            }
                        }) {
                            Some(_) => false,
                            None => true,
                        };

                        match addition_needed {
                            true => {
                                acc_event_type_scripts.push(current.clone());
                                acc_event_type_scripts.clone()
                            }
                            false => {
                                acc_event_type_scripts.clone()
                            }
                        }
                    },
                    None => {
                        // insert current into accumulator
                        let mut accumulator_copy= scripts_by_event_type_acc.clone();
                        let mut event_type_and_schema_to_insert = vec![current.clone()];
                        accumulator_copy.insert(event_type.clone(), event_type_and_schema_to_insert.clone());
                        // accumulator_copy
                        event_type_and_schema_to_insert
                    }
                };
                _acc.insert(event_type.clone(), event_schemas);
                _acc
            });
            updated_acc
        });

        Ok(Scripts{
            scripts_by_event_triggers,
            threads: None
        })
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

impl From<std::io::Error> for ScriptsError {
    fn from(value:std::io::Error) -> Self {
        ScriptsError::IoError(value)
    }
}


impl FromStr for ScriptsError {
    fn from_str(s: &str) -> Result<ScriptsError, ScriptsError> {
        Ok(ScriptsError::GenericMessage(s.to_string()))
    }
    type Err = ScriptsError;
}

#[derive(Deserialize, Debug, Clone)]
pub struct ScriptSchema {
    event_triggers: Vec<String>,
    file_name: String,
    failed: Option<bool>
}

pub enum ScriptsError {
    ConfigsError,
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    SpawnError(SpawnError),
    GenericMessage(String),
}

impl fmt::Display for ScriptsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptsError::ConfigsError => 
                write!(f, "error parsing event types as string"),
            ScriptsError::IoError(e) => 
                write!(f, "error parsing event types as string: {}", e),
            ScriptsError::JsonError(e) => 
                write!(f, "error parsing event types as a string: {}", e),
            ScriptsError::SpawnError(e) => 
                write!(f, "error parsing event types as a string: {}", e),
            ScriptsError::GenericMessage(e) => 
                write!(f, "error parsing event types as a string: {}", e)
        }
    }
}

impl From<serde_json::Error> for ScriptsError {
    fn from(value: serde_json::Error) -> Self {
        ScriptsError::JsonError(value)
    }
}

impl From<SpawnError> for ScriptsError {
    fn from(value:SpawnError) -> Self {
        ScriptsError::SpawnError(value)
    }
}

pub enum SpawnError {
    IoError(std::io::Error),
    ReadError(),
    ParseError(ParseError)
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnError::IoError(e) => 
                write!(f, "error parsing event types as string"),
            SpawnError::ReadError() => 
                write!(f, "error parsing event types as string"),
            SpawnError::ParseError(e) => 
                write!(f, "error parsing event types as a string: {}", e)
        }
    }
}

impl From<std::io::Error> for SpawnError {
    fn from(value:std::io::Error) -> Self {
        SpawnError::IoError(value)
    }
}

impl From<ParseError> for SpawnError {
    fn from(value: ParseError) -> Self {
        SpawnError::ParseError(value)
    }
}

pub trait Spawn {
    fn run(path: &String) -> Result<Child, ScriptsError> {
        info!("attempting to run script at: {}", path.clone().to_string());
        Ok(Command::new("sh")
            .arg("-C")
            .arg(path.to_string())
            .spawn()?)
    }

    // fn validate_scripts(path: &String) -> Result<bool, SpawnError> {
    //     let paths = fs::read_dir(path.clone())?;

    //     let all_executable = paths.fold(true, |valid_so_far, path|  {
            // if !valid_so_far {
            //     return valid_so_far
            // }
            // let current_valid = match path {
            //     Ok(p) => p,
            //     Err(e) => return false,
            // };
            // is_executable::is_executable(current_valid.path())
    //     });

    //     Ok(all_executable)
    // }
}

impl Spawn for Scripts {}