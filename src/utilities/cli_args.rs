use std::path::PathBuf;
use clap::Parser;
use log::LevelFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArgs {
    /// level of logging
    #[arg(short, long, default_value="error")]
    pub level: LevelFilter,
    /// path to watch
    #[arg(short, long, use_value_delimiter=true)]
    pub watch_path: Vec<PathBuf>,
}
