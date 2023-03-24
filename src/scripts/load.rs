use std::{fs, path::Path, collections::HashMap};
use log::{info, debug};
use notify::{Event, EventKind, event::AccessKind};
use crate::scripts::r#struct::ScriptJSON;
use crate::errors::script_errors::script_error::ScriptError;
use super::r#struct::{Scripts, Script, ScriptsByEventTrigger};

impl Scripts {
    pub fn get_by_event(&self, event: &Event) -> Vec<Script> {
        match self.scripts_by_event_triggers.get(&event.kind) { 
            Some(scripts) => scripts.clone(),
            None => return vec![]
        }
    }
    
    pub fn ingest_configs(configs_path: &String) -> Result<Self, ScriptError> {
        let configs_file = fs::read_to_string(format!("{}/scripts_config.json", configs_path)).map_err(|e| ScriptError::IoError(e))?;

        let files = serde_json::from_str::<Vec<ScriptJSON>>(&configs_file)?;

        for file in &files {
            let message = serde_json::to_string_pretty(&file);
            info!("script configuration: {:?}", message);
        }   

        match files.clone().into_iter().fold(true, |valid_so_far, current| {
            if !valid_so_far {
                return valid_so_far
            }
            let path_string = format!("./{}/{}", configs_path.clone(), current.file_name.clone());
            info!("path of identified script: {}", path_string);
            let as_path = Path::new(&path_string);
            is_executable::is_executable(as_path)
        }) {
            true => {},
            false => return Err(ScriptError::GenericMessage("unable to validate scripts folder".into()).into()),
        }
        let scripts_by_event_triggers = Self::cache_scripts_by_events(&files);        Ok(Scripts{
            scripts_by_event_triggers,
        })
    }

    fn cache_scripts_by_events(files: &Vec<ScriptJSON>) -> HashMap<EventKind, Vec<Script>> {
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

    fn update_schema_vec(event_type: &EventKind, script_json: ScriptJSON, scripts: &mut HashMap<EventKind, Vec<Script>>) -> Vec<Script> {
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
                let event_type_and_schema_to_insert = vec![script_json.clone().into()];
                scripts.insert(event_type.clone(), vec![script_json.clone().into()]);
                event_type_and_schema_to_insert
            }
        }
    }
}
