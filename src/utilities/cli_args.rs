use super::traits::Utilities;
use crate::errors::command_line_errors::enums::CommandLineError;
use clap::Parser;
use itertools::Itertools;
use log::{debug, LevelFilter};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArgs {
    /// level of logging
    #[arg(short, long, default_value = "error")]
    pub log_level: LevelFilter,
    /// path to configuration file - required
    #[arg(short, long)]
    pub script_folder: PathBuf,
}

impl Utilities for CommandLineArgs {}

impl CommandLineArgs {
    pub fn get_config_path(&self) -> Result<PathBuf, CommandLineError> {
        let possible_config_error = CommandLineError::ScriptConfigError(
            "unable to verify script configuration file".to_string(),
        );
        let config_path = self.script_folder.clone();
        let config_path_str = config_path.to_str().unwrap_or("");
        debug!("config path: {}", config_path_str);
        let config_dir = config_path.canonicalize()?.read_dir()?;
        let config_dir_files = config_dir.collect_vec();
        if !Self::dir_contains_file_type(&config_dir_files, &"yml".to_string()) {
            return Err(possible_config_error);
        }
        let config_file = Self::get_first_of_file_type(&config_dir_files, &"yml".to_string())
            .ok_or(possible_config_error)?;
        Ok(config_file)
    }
}
