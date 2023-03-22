#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(trait_alias)]
#![feature(async_closure)]
#![feature(is_some_and)]
use logger::{r#struct::Logger, error::ErrorLogging};
use watcher::{configs, watcher_scripts::WatcherScripts, init::Watcher};
use log::error;
mod logger;
mod watcher;


#[tokio::main]
async fn main() {
    match tokio::task::spawn(async {
        poll().await;
    }).await {
        Ok(_) => (),
        Err(_) => {
            Logger::log_error_string(&"error executing poll()".to_string());
        }
    }
}

async fn poll() {
    match tokio::spawn(async move {
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
        let watcher = Watcher::init(&watcher_scripts);

        match watcher.watch_handle {
            Ok(join_handle) => {
                match join_handle.await {
                    Ok(()) => {},
                    Err(e) => {
                        error!("{}", e.to_string());
                        panic!()
                    }
                }
            },
            Err(_) => {
                return
            }
        }
    }).await {
        Ok(_) => (),
        Err(_) => {
            Logger::log_error_string(&"error spawning loop".into());
        }
    }
}