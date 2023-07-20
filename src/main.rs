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

use std::path::{PathBuf, Path};
use clap::Parser;
use errors::watcher_errors::watcher_error::WatcherError;
use futures::future::try_join_all;
use logger::{structs::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging};
use runner::structs::Runner;
use scripts::structs::Scripts;
use watcher::structs::Watcher;
use utilities::{thread_types::{SpawnSender, UnsubscribeSender}, cli_args::CommandLineArgs};

use crate::utilities::set_process_lockfile::Lockfile;

#[tokio::main]
async fn main() {
    let args = CommandLineArgs::parse();
    Logger::on_load(args.log_level).unwrap();

    let config_path = args.get_config_path().unwrap();
    let config_path_clone = config_path.as_path();

    if let Err(e) = Lockfile::set(None, None) {
        Logger::log_debug_string(&e.to_string());
        Logger::log_error_string(&format!("unable to get a lock on the pid file. this is done to ensure only one instance is running at a time").to_string());
        panic!()
    };

    let runner = Runner::new().unwrap(); // if we cant get a runner up we should panic
    let script_task_spawn_channel = runner.spawn_channel.0.clone();
    let unsub_from_folder_channel = runner.unsubscribe_broadcast_channel.0.clone();

    let runner_task = runner.init();

    let watch_paths = match Scripts::watch_paths(&config_path) {
        Ok(p) => p,
        Err(e) => {
            Logger::log_error_string(&e.to_string());
            panic!()
        }
    };

    let watchers:Vec<_> = watch_paths.iter().map(|watch_path| {
        initialize_watchers(watch_path, config_path_clone, script_task_spawn_channel.clone(), unsub_from_folder_channel.clone())
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

async fn initialize_watchers(watch_path:&PathBuf, scripts_config_path: &Path, spawn_channel: SpawnSender, unsubscribe_channel: UnsubscribeSender) -> Result<(), WatcherError>{
    let watcher_scripts = match Scripts::load(watch_path, scripts_config_path) {
        Ok(script_records) => script_records,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };

    let watcher = Watcher::new()?;

    Ok(watcher.start(spawn_channel, unsubscribe_channel, watch_path.clone(), &watcher_scripts).await?)
}
