use std::{error, fmt::{self}, str::FromStr};
use serde::{ Deserialize, Serialize };

use super::{event_structs::{ SyncthingEvent, EventTypes }, client};

#[derive(Debug, Clone, Serialize, Deserialize,)]
struct SyncthingError;

impl fmt::Display for SyncthingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error while talking to syncthing")
    }
}

impl error::Error for SyncthingError {}

pub type SyncthingApiError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct SyncthingApi {
    pub client: reqwest::Client,
    pub seen: Vec<u16>,
    pub last_seen: Option<u16>,
}

impl SyncthingApi {
    
    pub fn new () -> self::SyncthingApi {
        SyncthingApi {
            client: client::Client { auth_key: "oauUuuTMbTspjiKY5jyVnrh5Lf3a23Sj".to_string() }.new(),
            seen: [].to_vec(),
            last_seen: None,
        }
    }

    pub fn update_seen(&mut self, events: &Vec<SyncthingEvent>) -> &self::SyncthingApi {
        let mut existing_seen_ids = self.seen.clone();
        let mut new_ids = self.map_ids(events).clone();
        
        let compiled_ids:Vec<u16> = self.merge_ids(&mut existing_seen_ids, &mut new_ids);
        let compiled_ids_len = compiled_ids.len();

        let trimmed_and_compiled = match compiled_ids_len > 100 {
            true => {
                println!("all seen ids are of length: {}, cutting seen down to size", compiled_ids_len);
                let to_slice = compiled_ids.as_slice();
                let (_, trimmed) = to_slice.split_at(compiled_ids_len - 100);
                trimmed.to_vec()
            },
            false => compiled_ids
        };

        self.last_seen = trimmed_and_compiled.last().copied();
        self.seen = trimmed_and_compiled;

        self
    }

    pub async fn update(&mut self) -> Result<&self::SyncthingApi, SyncthingApiError> {
        let new_events = match self.fetch_events().await {
            Ok(events) => events,
            Err(e) => {
                let error_msg = e.to_string();
                return Err(error_msg.into())
            }
        };

        let local_index_updated_event = match EventTypes::from_str("LocalIndexUpdated") {
            Ok(event_type) => event_type,
            Err(e) => {
                return Err(e.into())
            }
        };

        let local_index_updated_events = self.filter_events(&new_events, &local_index_updated_event);

        Ok(self.update_seen(&local_index_updated_events))
    }
}
