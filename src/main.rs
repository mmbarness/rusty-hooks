#![feature(io_error_more)]
mod errors;
mod health_reporter;
mod watcher;
mod runner;
mod scripts;
mod utilities;

use std::path::{PathBuf, Path};
use clap::Parser;
use errors::watcher_errors::watcher_error::WatcherError;
use futures::future::try_join_all;
use log::{debug, info, error};
use runner::structs::Runner;
use scripts::structs::Scripts;
use watcher::structs::Watcher;
use utilities::{thread_types::{SpawnSender, UnsubscribeSender}, cli_args::CommandLineArgs};

use crate::utilities::set_process_lockfile::Lockfile;

#[tokio::main]
async fn main() {
    let args = CommandLineArgs::parse();
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("starting rusty hooks....");
    let config_path = match args.get_config_path() {
        Ok(c) => c,
        Err(e) => {
            debug!("{}", e.to_string());
            panic!()
        }
    };
    let config_path_clone = config_path.as_path();

    if let Err(e) = Lockfile::set(None, None) {
        debug!("{}", e.to_string());
        error!("unable to get a lock on the pid file. this is done to ensure only one instance is running at a time");
        panic!()
    };

    let runner = Runner::new().unwrap(); // if we cant get a runner up we should panic
    let script_task_spawn_channel = runner.spawn_channel.0.clone();
    let unsub_from_folder_channel = runner.unsubscribe_broadcast_channel.0.clone();

    let runner_task = runner.init();

    let watch_paths = match Scripts::all_watch_paths(&config_path) {
        Ok(p) => p,
        Err(e) => {
            error!("{}", e.to_string());
            panic!()
        }
    };

    let watchers:Vec<_> = watch_paths.iter().map(|watch_path| {
        initialize_watchers(watch_path, config_path_clone, script_task_spawn_channel.clone(), unsub_from_folder_channel.clone())
    }).collect();

    let awaited_watchers = try_join_all(watchers);

    let health_reporter = health_reporter::HealthReporter::new().unwrap();
    let report_task = health_reporter.begin_reporting();

    utilities::set_process_lockfile::Lockfile::set(None, None).unwrap();

    tokio::select! {
        b = awaited_watchers => {
            match b {
                Ok(_) => {
                    info!("event watcher tasks exited, cleaning up other tasks and exiting")
                },
                Err(e) => {
                    error!("event watchers failed: {}", e);
                    info!("cleaning up other tasks and exiting");
                }
            }
        },
        b = report_task => {
            match b {
                Ok(_) => {
                    info!("health report task exited.")
                },
                Err(e) => {
                    error!("health report task exited with an error: {}", e);
                }
            }
        },
        a = runner_task => {
            match a {
                Ok(_) => {
                    info!("runner task exited, cleaning up other tasks and exiting")
                },
                Err(e) => {
                    error!("runner task failed: {}", e);
                    info!("cleaning up other tasks and exiting");
                }
            }
        }
    }
}

async fn initialize_watchers(watch_path:&PathBuf, scripts_config_path: &Path, spawn_channel: SpawnSender, unsubscribe_channel: UnsubscribeSender) -> Result<(), WatcherError>{
    let watcher_scripts = match Scripts::by_watch_path(watch_path, scripts_config_path) {
        Ok(s) => s,
        Err(e) => {
            error!("error loading configs: {}", e);
            panic!()
        }
    };

    let watcher = Watcher::new()?;

    Ok(watcher.start(spawn_channel, unsubscribe_channel, watch_path.clone(), &watcher_scripts).await?)
}
