use std::{str::FromStr};
use crate::syncthing::logger::{Logger, InfoLogging};

use super::{
    event_structs::{
        SyncthingEvent,
        EventTypes
    },
    client::{
        self,
        Client
    },
    errors::{
        SyncthingError,
        EventTypesError
    },
    configs::Configs, logger::DebugLogging
};

#[derive(Debug, Clone)]
pub struct SyncthingApi {
    pub client: Client,
    pub configs: Configs,
    pub seen: Vec<u16>,
    pub last_seen: Option<u16>,
}

impl SyncthingApi {
    
    pub fn new (configs: Configs) -> self::SyncthingApi {
        SyncthingApi {
            client: client::Client::new(&configs.auth_key, &configs.address, &configs.port),
            configs,
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
                Logger::log_info_string(&format!("all seen ids are of length: {}, cutting seen down to size", compiled_ids_len));
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

    pub async fn update(&mut self) -> Result<&self::SyncthingApi, SyncthingError> {
        let new_events = self.fetch_events().await?;

        let local_index_updated_event = match EventTypes::from_str("LocalIndexUpdated") {
            Ok(event_type) => event_type,
            Err(e) => return Err(EventTypesError::ParseString(e).into())
        };

        let local_index_updated_events = self.filter_events(&new_events, &local_index_updated_event);

        match local_index_updated_events.last() {
            Some(event) => {
                Logger::log_debug_string(&format!("found new event of type LocalIndexUpdated: {}", serde_json::to_string_pretty(&event).unwrap()));
            }, 
            None => {}
        }

        let most_recent_folder_state = self.examine_folder_summary(&new_events)?;

        Ok(self.update_seen(&local_index_updated_events))
    }
}
