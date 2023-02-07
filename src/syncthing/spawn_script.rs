use std::{process::{Command, Child}, fs::{self, DirEntry, ReadDir}, str::FromStr, thread::{self, JoinHandle, Thread}, time::Duration, sync::{mpsc::channel, Mutex, Arc}};
use log::{info, debug};
use is_executable::IsExecutable;
use serde::Deserialize;
use threadpool::ThreadPool;
use super::errors::SyncthingError;

#[derive(Debug)]
pub struct Scripts {
    files: Vec<ScriptSchema>,
    threads: Vec<JoinHandle<Result<Child, ScriptsError>>>,
}

impl Scripts {
    pub fn ingest_configs() -> Result<Vec<ScriptSchema>, ScriptsError> {
        let configs_file = fs::read_to_string("./scripts/scripts_config.json")?;

        let files = serde_json::from_str::<Vec<ScriptSchema>>(&configs_file)?;

        match files.clone().into_iter().fold(true, |valid_so_far, current| {
            if !valid_so_far {
                return valid_so_far
            }
            let path = format!("./scripts/{}", current.file_name.clone());
            is_executable::is_executable(path)
        }) {
            true => {},
            false => return Err(ScriptsError::GenericMessage("unable to validate scripts folder".to_string())),
        }

        Ok(files)
    }

    pub fn start(files:Vec<ScriptSchema>) -> Result<Self, ScriptsError> {
        let threads:Vec<JoinHandle<Result<Child, ScriptsError>>> = files.iter().clone().map(move |file| {
            let file_arc = Arc::new(file.clone());
            thread::spawn(move ||{
                let frequency = file_arc.execution_frequency.clone();
                let path = file_arc.file_name.clone();
                let process = match Self::run(&path) {
                    Ok(child) => Ok(child),
                    Err(e) => Err(e)
                };
                thread::sleep(Duration::from_millis(frequency));
                process
            })
        }).collect();

        Ok(Scripts{
            files,
            threads,
        })
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
    execution_frequency: u64,
    file_name: String,
    run_delay: u8,
    failed: Option<bool>
}

pub enum ScriptsError {
    ConfigsError,
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    SpawnError(SpawnError),
    GenericMessage(String),
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
    ReadError()
}

impl From<std::io::Error> for SpawnError {
    fn from(value:std::io::Error) -> Self {
        SpawnError::IoError(value)
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