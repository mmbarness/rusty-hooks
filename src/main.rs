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
        let mut interval = time::interval(time::Duration::from_secs(10));
        let syncthing_api = syncthing::api::SyncthingApi::new();
        loop {
            println!("polling...");
            let new_events = match syncthing_api.fetch_events().await {
                Ok(events) => events,
                Err(e) => {
                    let message = e.to_string();
                    println!("{}", message);
                    return;
                }
            };
            interval.tick().await;
            println!("{}", serde_json::to_string_pretty(&new_events).unwrap());
        }
    }).await {
        Ok(_) => (),
        Err(_) => {
            println!("error spawning loop")
        }
    }
}