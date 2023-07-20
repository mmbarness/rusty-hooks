use std::path::{PathBuf, Path};
use std::{fs,collections::HashMap};
use anyhow::anyhow;
use itertools::Itertools;
use itertools::FoldWhile::{Continue, Done};
use notify::{EventKind, event::AccessKind};
use crate::logger::debug::DebugLogging;
use crate::logger::error::ErrorLogging;
use crate::logger::structs::Logger;
use crate::utilities::traits::Utilities;
use crate::scripts::structs::ScriptJSON;
use crate::errors::script_errors::script_error::{ScriptConfigError, ScriptError};
use super::structs::{Scripts, Script, ScriptsByEventTrigger};

impl Scripts {
    pub fn get_by_event(&self, event_kind: &EventKind) -> Vec<Script> {
        match self.scripts_by_event_triggers.get(&event_kind) { 
            Some(scripts) => scripts.clone(),
            None => return vec![]
        }
    }

    pub fn validate_scripts(watch_path: &PathBuf, unvalidated_scripts: Vec<ScriptJSON>, script_directory: &String) -> Result<Vec<Script>, ScriptConfigError> {
        let script_validations:Vec<Result<(bool, PathBuf), std::io::Error>> = unvalidated_scripts.iter().map(|script| {
            let script_path = Self::build_path(&vec![&script_directory, &script.file_name]);
            let io_error_kind = std::io::ErrorKind::InvalidFilename;
            let script_path_io_error = std::io::Error::new(
                io_error_kind, 
                format!(
                    "unable to find script: {}, at path: {}", script.file_name, Self::format_unvalidated_path(&vec![&"./".to_string(), &script_directory, &script.file_name]).to_string()
                )
            );
            let watch_paths_match = watch_path.clone() == std::path::Path::new(&script.watch_path).to_path_buf();
            match (script_path, watch_paths_match) {
                (Some(path), true) => Ok((true, path)),
                (None, true) => Err(script_path_io_error),
                (Some(path), false) => Ok((false, path)),
                (None, false) => Err(script_path_io_error)
            }
        }).collect();

        let scripts_matching_watch_path:Vec<&Result<(bool, PathBuf), std::io::Error>> = script_validations.iter().filter(|script| {
            script.as_ref().is_ok_and(|s| s.0)
         }).collect();

        let errors_found = script_validations.iter().any(|ele| ele.is_err());

        match scripts_matching_watch_path.len() {
            a if a <= 0 => {
                let io_error_kind = std::io::ErrorKind::InvalidFilename;
                let io_error = std::io::Error::new(
                    io_error_kind, 
                    "no scripts matched the provided watch paths"
                );
                return Err(ScriptConfigError::IoError(io_error))
            }
            _ => {}
        };

        if errors_found {
            for script in script_validations {
                let _ = script.as_ref().inspect_err(|e| Logger::log_error_string(&e.to_string()));
            }
            let io_error_kind = std::io::ErrorKind::InvalidFilename;
            let io_error = std::io::Error::new(
                io_error_kind, 
                "script validation error"
            );
            Err(ScriptConfigError::IoError(io_error))
        } else {
            Ok(unvalidated_scripts.iter().map_into().collect_vec())
        }
    }

    pub fn watch_paths(config_path: &Path) -> Result<Vec<PathBuf>, ScriptError> {
        let configs_file = fs::read_to_string(config_path.clone()).map_err(ScriptError::IoError)?;

        let files = serde_json::from_str::<Vec<ScriptJSON>>(&configs_file).map_err(ScriptConfigError::JsonError)?;

        let mut bad_path: Option<PathBuf> = None;

        let watch_paths = files.iter().fold_while(vec![], |mut acc: Vec<PathBuf>, script:&ScriptJSON| {
            let path = Path::new(&script.watch_path);
            let can_read = path.read_dir();

            match (script.enabled, path.is_dir(), can_read){
                (true, true, Ok(_)) => {
                    acc.push(path.to_path_buf());
                    Continue(acc)
                },
                (true, true, Err(e)) => {
                    Logger::log_debug_string(&format!("error reading path entries: {}",
                        e.to_string()
                    ));
                    bad_path = Some(path.to_path_buf());
                    Done(acc)
                },
                (true, false, _) => {
                    Logger::log_debug_string(&format!("provided path is not a directory: {:?}",
                        path.to_str()
                    ));
                    bad_path = Some(path.to_path_buf());
                    Done(acc)
                }
                (false, _, _) => {
                    Continue(acc)
                },
            }
        }).into_inner();

        match bad_path.is_some() {
            true => {
                let path = bad_path.unwrap();
                let path_string = path.to_str().unwrap_or("unable to parse");
                let message_string = format!("error with a provided watch path: {}", path_string);
                let config_error = Into::<ScriptConfigError>::into(anyhow!(message_string));
                return Err(config_error.into())
            },
            _ => {
                Logger::log_debug_string(&"all paths validated".to_string());
                return Ok(watch_paths)
            }
        }
    }
    
    pub fn load(watch_path: &PathBuf, scripts_config_path: &Path) -> Result<Self, ScriptError> {
        let config_path_buf = scripts_config_path.to_path_buf();
        let script_config_path_str = scripts_config_path.to_str().unwrap_or("");

        let script_directory_path = Scripts::get_parent_dir_of_file(&config_path_buf)
            .ok_or(ScriptError::ConfigError("unable to parse parent directory of provided script configuration path".to_string().into()))?;
        let script_directory_path_string = script_directory_path.to_str().ok_or(ScriptError::ConfigError("unable to parse parent directory of provided script configuration path".to_string().into()))?.to_string();

        let configs_file = fs::read_to_string(script_config_path_str.clone()).map_err(ScriptError::IoError)?;

        let files = serde_json::from_str::<Vec<ScriptJSON>>(&configs_file).map_err(ScriptConfigError::JsonError)?;

        let validated_scripts = Self::validate_scripts(watch_path, files, &script_directory_path_string)?;

        let watch_paths:Vec<PathBuf> = validated_scripts.iter().map(|s| {
            s.watch_path.clone()
        }).collect();

        let filtered_by_watch_path:Vec<Script> = validated_scripts.iter().filter(|script| {
            watch_path.clone() == std::path::Path::new(&script.watch_path).to_path_buf()
        }).map(|s| s.to_owned()).collect();

        Logger::log_debug_string(&format!("{} scripts found that match provided watch path", filtered_by_watch_path.len()));

        let scripts_by_event_triggers = Self::cache_scripts_by_events(&filtered_by_watch_path);        
        
        Ok(Scripts{
            scripts_by_event_triggers,
            watch_paths,
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
        Logger::log_debug_string(&format!("deciding whether to insert script {:?}", serde_json::to_string_pretty(&script_json.file_name).unwrap_or("unable to parse script file name".to_string())));
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
