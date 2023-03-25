use std::path::PathBuf;
use std::{fs,collections::HashMap};
use itertools::Itertools;
use log::{info, debug};
use notify::{Event, EventKind, event::AccessKind};
use crate::logger::error::ErrorLogging;
use crate::logger::structs::Logger;
use crate::utilities::traits::Utilities;
use crate::scripts::structs::ScriptJSON;
use crate::errors::script_errors::script_error::{ConfigError, ScriptError};
use super::structs::{Scripts, Script, ScriptsByEventTrigger};

impl Scripts {
    pub fn get_by_event(&self, event: &Event) -> Vec<Script> {
        match self.scripts_by_event_triggers.get(&event.kind) { 
            Some(scripts) => scripts.clone(),
            None => return vec![]
        }
    }

    pub fn validate_scripts(unvalidated_scripts: Vec<ScriptJSON>, configs_path: &String) -> Result<Vec<Script>, ConfigError> {
        let script_validations:Vec<Result<(bool, PathBuf), std::io::Error>> = unvalidated_scripts.iter().map(|script| {
            let script_path = Self::build_path(&vec![&"./".to_string(), &configs_path, &script.file_name]);
            let io_error_kind = std::io::ErrorKind::InvalidFilename;
            let io_error = std::io::Error::new(
                io_error_kind, 
                format!(
                    "unable to find script: {}, at path: {}", script.file_name, Self::format_unvalidated_path(&vec![&"./".to_string(), &configs_path, &script.file_name]).to_string()
                )
            );
            match script_path {
                Some(path) => Ok((true, path)),
                None => Err(io_error)
            }
        }).collect();

        let errors_found = script_validations.iter().any(|ele| ele.is_err()); 

        if errors_found {
            for script in script_validations {
                let _ = script.as_ref().inspect_err(|e| Logger::log_error_string(&e.to_string()));
            }
            let io_error_kind = std::io::ErrorKind::InvalidFilename;
            let io_error = std::io::Error::new(
                io_error_kind, 
                "script validation error"
            );
            Err(ConfigError::IoError(io_error))
        } else {
            Ok(unvalidated_scripts.iter().map_into().collect_vec())
        }
    }
    
    pub fn ingest_configs(configs_path: &String) -> Result<Self, ScriptError> {
        let configs_file = fs::read_to_string(format!("{}/scripts_config.json", configs_path)).map_err(ScriptError::IoError)?;

        let files = serde_json::from_str::<Vec<ScriptJSON>>(&configs_file).map_err(ConfigError::JsonError)?;

        for file in &files {
            let message = serde_json::to_string_pretty(&file);
            info!("script configuration: {:?}", message);
        }

        let validated_scripts = Self::validate_scripts(files, configs_path)?;
        let scripts_by_event_triggers = Self::cache_scripts_by_events(&validated_scripts);        
        
        Ok(Scripts{
            scripts_by_event_triggers,
        })
    }

    fn cache_scripts_by_events(files: &Vec<Script>) -> HashMap<EventKind, Vec<Script>> {
        let acc_int:ScriptsByEventTrigger = HashMap::new();
        let files_iter = files.clone().into_iter();
        // iterate over each file and reduce to hashmap of event type and associated scripts to run
        files_iter.fold(acc_int, |scripts_by_event_type_acc, current_script| {
            let current_event_triggers = &current_script.event_triggers;
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
                let event_schemas = Self::update_schema_vec(&event_kind, current_script.clone(), &mut scripts_by_event_type_acc);
                scripts_by_event_type_acc.insert(event_kind.clone(), event_schemas);
                scripts_by_event_type_acc
            })
        })
    }

    fn update_schema_vec(event_type: &EventKind, script_json: Script, scripts: &mut HashMap<EventKind, Vec<Script>>) -> Vec<Script> {
        debug!("deciding whether to insert script {:?}", serde_json::to_string_pretty(&script_json.file_name));
        match scripts.get_mut(event_type) {
            Some(acc_event_type_scripts) => { // array of scripts attached to event_type
                let script_exists = acc_event_type_scripts.into_iter().any(|script| {
                    script_json.file_name.eq_ignore_ascii_case(&script.file_name)
                });
                match script_exists {
                    true => {
                        acc_event_type_scripts.to_vec()
                    }
                    false => {
                        acc_event_type_scripts.push(script_json.into());
                        acc_event_type_scripts.to_vec()
                    }
                }
            },
            None => {
                let event_type_and_schema_to_insert = vec![script_json.clone()];
                scripts.insert(event_type.clone(), vec![script_json.clone()]);
                event_type_and_schema_to_insert
            }
        }
    }
}
