#![feature(io_error_more)]
#![feature(result_option_inspect)]
#![feature(fs_try_exists)]
#![feature(is_some_and)]
mod logger;
mod errors;
mod watcher;
mod runner;
mod scripts;
mod utilities;

use std::path::PathBuf;

use clap::Parser;
use errors::watcher_errors::watcher_error::WatcherError;
use futures::future::try_join_all;
use logger::{structs::Logger, error::ErrorLogging, info::InfoLogging};
use runner::structs::Runner;
use scripts::structs::Scripts;
use watcher::{structs::Watcher};
use utilities::{thread_types::{SpawnSender, UnsubscribeSender}, cli_args::CommandLineArgs};

#[tokio::main]
async fn main() {
    let runner = Runner::new().unwrap(); // if we cant get a runner up we should panic
    
    let spawn_channel = runner.spawn_channel.0.clone();
    let unsubscribe_channel = runner.unsubscribe_broadcast_channel.0.clone();

    let args = CommandLineArgs::parse();
    Logger::on_load(args.log_level);

    let runner_task = runner.init();

    let watch_paths = match Scripts::watch_paths(args.script_config) {
        Ok(p) => p,
        Err(e) => {
            Logger::log_error_string(&e.to_string());
            panic!()
        }
    };

    let watchers:Vec<_> = watch_paths.iter().map(|watch_path| {
        initialize_watchers(watch_path, spawn_channel.clone(), unsubscribe_channel.clone())
    }).collect();

    let awaited_watchers = try_join_all(watchers);

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
        b = awaited_watchers => {
            match b {
                Ok(_) => {
                    Logger::log_info_string(&format!("event watcher tasks exited, cleaning up other tasks and exiting").to_string())
                },
                Err(e) => {
                    Logger::log_error_string(&format!("event watchers failed: {}", e).to_string());
                    Logger::log_info_string(&format!("cleaning up other tasks and exiting"));
                }
            }
        }
    }
}

async fn initialize_watchers(watch_path:&PathBuf, spawn_channel: SpawnSender, unsubscribe_channel: UnsubscribeSender) -> Result<(), WatcherError>{
    let watcher_scripts = match Scripts::load(watch_path) {
        Ok(script_records) => script_records,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };

    let watcher = Watcher::new()?;

    Ok(watcher.start(spawn_channel, unsubscribe_channel, watch_path.clone(), &watcher_scripts).await?)
}