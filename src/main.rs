#![feature(trait_alias)]
#![feature(io_error_more)]
#![feature(result_option_inspect)]
#![feature(fs_try_exists)]
mod logger;
mod errors;
mod watcher;
mod runner;
mod scripts;
mod utilities;

use std::path::PathBuf;

use clap::Parser;
use errors::watcher_errors::{watcher_error::WatcherError, event_error::EventError};
use log::{Level, LevelFilter};
use logger::{structs::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging};
use runner::structs::Runner;
use scripts::structs::Scripts;
use watcher::{configs, structs::Watcher};
use utilities::thread_types::{SpawnSender, UnsubscribeSender};

#[tokio::main]
async fn main() {
    let runner = Runner::new().unwrap(); // if we cant get a runner up we should panic
    
    let spawn_channel = runner.spawn_channel.0.clone();
    let unsubscribe_channel = runner.unsubscribe_broadcast_channel.0.clone();

    let runner_task = tokio::spawn(async move {
        runner.init().await
    });
    
    let watcher_task = initialize_watchers(spawn_channel, unsubscribe_channel);

    tokio::select! {
        a = runner_task => {
            match a {
                Ok(_) => {
                    Logger::log_info_string(&format!("runner task exited, cleaning up other tasks and exiting").to_string())
                },
                Err(e) => {
                    Logger::log_error_string(&format!("runner task failed: {}", e).to_string());
                    Logger::log_info_string(&format!("cleaning up other tasks and exiting"));
                }
            }
        },
        b = watcher_task => {
            match b {
                Ok(_) => {
                    Logger::log_info_string(&format!("event watcher task exited, cleaning up other tasks and exiting").to_string())
                },
                Err(e) => {
                    Logger::log_error_string(&format!("event watcher failed: {}", e).to_string());
                    Logger::log_info_string(&format!("cleaning up other tasks and exiting"));
                }
            }
        }
    }

}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// level of logging
    #[arg(short, long, default_value="error")]
    level: LevelFilter,
    /// path to watch
    #[arg(short, long)]
    watch_path: PathBuf,
}

async fn initialize_watchers(spawn_channel: SpawnSender, unsubscribe_channel: UnsubscribeSender) -> Result<(), WatcherError>{
    let args = Args::parse();
    Logger::on_load(args.level);
    let api_configs = match configs::Configs::load() {
        Ok(c) => c,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };
    
    let scripts_path = api_configs.scripts_path.clone();
    
    let watcher_scripts = match Scripts::ingest_configs(&scripts_path) {
        Ok(script_records) => script_records,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };

    let watcher = Watcher::new()?;

    Ok(watcher.start(spawn_channel, unsubscribe_channel, args.watch_path, &watcher_scripts).await.map_err(|e| { EventError::NotifyError(e) })?)

}