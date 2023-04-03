use std::path::PathBuf;
use clap::Parser;
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArgs {
    /// level of logging
    #[arg(short, long, default_value="error")]
    pub log_level: LevelFilter,
    #[arg(short, long)]
    pub script_config: Option<PathBuf>,
}
