use std::{process::{Command, Child}, fs::{self}};
use log::{info};
use crate::syncthing::errors::SpawnError;
use super::{errors::{ScriptsError}, scripts::Scripts};

pub trait Spawn {
    fn run(path: &String) -> Result<Child, ScriptsError> {
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

    // fn validate_threads(threads: Vec<Threads>) -> () {
    //     let filtered_threads:Vec<Vec<JoinHandle<Result<Child, ScriptsError>>>> = threads.into_iter().filter(|thread_option| {
    //         match thread_option {
    //             Some(thread) => true,
    //             None => false,
    //         }
    //     }).collect();
    //     let flattened_threads:Option<Threads> = filtered_threads.iter().flatten().collect();
    // }
}

impl Spawn for Scripts {}
