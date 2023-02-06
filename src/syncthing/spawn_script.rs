use std::process::Command;
use super::errors::SyncthingError;

pub struct Scripts {}

pub trait Spawn {
    fn run(path: &String) -> Result<(), SyncthingError> {
        match Command::new("sh")
            .arg("-C")
            .arg(path.to_string())
            .spawn() {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into())
            }
    }
}

impl Spawn for Scripts {}