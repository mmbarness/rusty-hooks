use super::{
    api::{
        SyncthingApi,
        SyncthingApiError
    },
    event_structs::{
        SyncthingEvent,
        EventTypes
    }
};

impl SyncthingApi {
    pub async fn fetch_events(&self) -> Result<Vec<SyncthingEvent>, SyncthingApiError> {
        let url = match self.last_seen {
            Some(num) => format!("http://localhost:8384/rest/events?since={}", num.to_string()),
            None => "http://localhost:8384/rest/events".to_string(),
        };

        let json_str = match self.client.get(url).send().await {
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

    pub fn filter_events(&self, all_events: &Vec<SyncthingEvent>, event_type: &EventTypes) -> Vec<SyncthingEvent> {
        let binding = event_type.clone();
        let event_type_str = binding.as_ref();
        all_events
            .into_iter()
            .filter(|event| event.r#type == event_type_str)
            .map(|event| event.clone().clone())
            .collect()
    }

    pub fn map_ids(&self, events: &Vec<SyncthingEvent>) -> Vec<u16> {
        events.into_iter().map(|event| {
            return event.id.clone()
        }).collect()
    }

    pub fn merge_ids(&self, old: & mut Vec<u16>, new: & mut Vec<u16>) -> Vec<u16> {
        let mut compiled_ids:Vec<u16> = vec![];
        compiled_ids.append(old);
        compiled_ids.append(new);
        compiled_ids.sort();
        compiled_ids.dedup();
        compiled_ids
    }
}