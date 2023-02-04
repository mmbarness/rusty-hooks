#![feature(provide_any)]
#![feature(error_generic_member_access)]
#![feature(trait_alias)]
use syncthing::configs;
use tokio::{ time };

mod syncthing;

#[tokio::main]
async fn main() {
    match tokio::task::spawn(async {
        poll().await;
    }).await {
        Ok(_) => (),
        Err(_) => {
            println!("error executing poll()")
        }
    }
}

async fn poll() {
    match tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_secs(30));
        let configs = match configs::Configs::load() {
            Ok(c) => c,
            Err(e) => {
                println!("error loading configs: {}", e.to_string());
                panic!()
            }
        };
        let mut syncthing_api = syncthing::api::SyncthingApi::new(configs);
        loop {
            println!("polling...");
            match syncthing_api.update().await {
                Ok(events) => events,
                Err(e) => {
                    let message = e.to_string();
                    println!("{}", message);
                    return;
                }
            };
            interval.tick().await;
        }
    }).await {
        Ok(_) => (),
        Err(_) => {
            println!("error spawning loop")
        }
    }
}