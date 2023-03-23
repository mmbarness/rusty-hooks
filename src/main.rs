#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(trait_alias)]
#![feature(async_closure)]
#![feature(is_some_and)]
mod logger;
mod errors;
mod watcher;
mod runner;
mod utilities;

use logger::{r#struct::Logger, error::ErrorLogging};
use runner::structs::Runner;
use watcher::{configs, watcher_scripts::WatcherScripts, structs::Watcher};
use utilities::thread_types::SubscribeSender;

#[tokio::main]
async fn main() {
    let runner = Runner::new();
    
    let spawn_channel = runner.spawn_channel.0.clone();

    let runner_task = tokio::spawn(async move {
        runner.init().await
    });
    
    initialize_watchers(spawn_channel).await;

    runner_task.abort();
}

async fn initialize_watchers(spawn_channel: SubscribeSender) {
    Logger::on_load();
    let api_configs = match configs::Configs::load() {
        Ok(c) => c,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };
    
    let scripts_path = api_configs.scripts_path.clone();
    
    let watcher_scripts = match WatcherScripts::ingest_configs(&scripts_path) {
        Ok(script_records) => script_records,
        Err(e) => {
            Logger::log_error_string(&format!("error loading configs: {}", e.to_string()));
            panic!()
        }
    };
    
    let root_watch_path = std::env::args()
        .nth(2)
        .expect("Argument 1 needs to be a path");

    let watcher = Watcher::new();

    match watcher.start(spawn_channel, root_watch_path, &watcher_scripts).await {
        Ok(_) => {

        },
        Err(e) => {
            Logger::log_error_string(&e.to_string())
        }
    }

}