use std::{error, fmt};
use serde::{ Deserialize, Serialize };

use super::{event_structs::{ SyncthingEvent }, client};

#[derive(Debug, Clone, Serialize, Deserialize,)]
struct SyncthingError;

impl fmt::Display for SyncthingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error while talking to syncthing")
    }
}

impl error::Error for SyncthingError {}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct SyncthingApi {
    pub client: reqwest::Client,
    pub seen: Vec<String>,
}

impl SyncthingApi {
    
    pub fn new () -> self::SyncthingApi {
        SyncthingApi {
            client: client::Client { auth_key: "oauUuuTMbTspjiKY5jyVnrh5Lf3a23Sj".to_string() }.new(),
            seen: [].to_vec(),
        }
    }

    pub async fn fetch_events(&self) -> Result<Vec<SyncthingEvent>, Error> {
        let json_str = match self.client.get("http://localhost:8384/rest/events").send().await {
            Ok(valid_response) => valid_response.text().await?,
            Err(e) => {
                let msg = e.to_string();
                println!("error converting resp to string, {}", msg);
                return Err(msg.into())
            }
        };
    
        let json_data:Vec<SyncthingEvent> = match serde_json::from_str::<Vec<SyncthingEvent>>(&json_str) {
            Ok(event_json) => event_json,
            Err(e) => {
                let msg = e.to_string();
                println!("json str: {}", json_str);
                println!("error converting resp to json, {}", msg);
                return Err(msg.into())
            }
        };
    
        Ok(json_data)
    }
}
