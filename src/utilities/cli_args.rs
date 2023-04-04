use std::path::PathBuf;
use clap::Parser;
use log::LevelFilter;

use crate::errors::command_line_errors::enums::CommandLineError;

use super::traits::Utilities;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArgs {
    /// level of logging
    #[arg(short, long, default_value="error")]
    pub log_level: LevelFilter,
    /// path to configuration file - required
    #[arg(short, long)]
    pub script_config: PathBuf,
}

impl Utilities for CommandLineArgs {}

impl CommandLineArgs {
    pub fn verify_config_path(&self) -> Result<(), CommandLineError> {
        let config_path = self.script_config.clone();
        let _ = CommandLineArgs::get_parent_dir_of_file(&config_path)
            .ok_or(CommandLineError::ScriptConfigError(format!("config path is invalid")))?;
        Ok(())
    }
}