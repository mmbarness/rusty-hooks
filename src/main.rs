#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(trait_alias)]
use syncthing::{configs, logger::{Logger, ErrorLogging}};
use tokio::{ time };
use log::{error, info};
mod syncthing;

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
        let configs = match configs::Configs::load() {
            Ok(c) => c,
            Err(e) => {
                error!("error loading configs: {}", e.to_string());
                panic!()
            }
        };
        let mut interval = time::interval(time::Duration::from_secs(configs.request_interval.clone()));
        let mut syncthing_api = syncthing::api::SyncthingApi::new(configs);
        info!("beginning to poll...");
        loop {
            match syncthing_api.update().await {
                Ok(events) => events,
                Err(e) => {
                    let message = e.to_string();
                    error!("{}", message);
                    return;
                }
            };
            interval.tick().await;
        }
    }).await {
        Ok(_) => (),
        Err(_) => {
            error!("error spawning loop")
        }
    }
}