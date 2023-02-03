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
        let mut syncthing_api = syncthing::api::SyncthingApi::new();
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