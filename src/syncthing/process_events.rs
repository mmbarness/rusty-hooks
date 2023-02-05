use std::{
    str::FromStr
};
use super::{
    api::{
        SyncthingApi,
    },
    errors::{SyncthingError, EventTypesError},
    event_structs::{
        SyncthingEvent,
        EventTypes, FolderState
    }
};
use crate::syncthing::logger::{Logger, DebugLogging};

impl SyncthingApi {

    pub fn examine_folder_summary(&self, all_events: &Vec<SyncthingEvent>) -> Result<FolderState, SyncthingError> {
        let folder_summary = match EventTypes::from_str("FolderSummary") {
            Ok(event_type) => event_type,
            Err(e) => {
                let event_type_error = EventTypesError::ParseString(e.into());
                return Err(SyncthingError::EventTypeError(event_type_error))
            }
        };

        let folder_summaries = self.filter_events(all_events, &folder_summary);

        let last_folder_summary = match folder_summaries.last().ok_or(SyncthingError::NoNewEvents) {
            Ok(most_recent) => most_recent.clone(),
            Err(e) => {
                return Err(e);
            }
        };

        let last_folder_state = match last_folder_summary.data {
            EventTypes::FolderSummary(f) => match FolderState::from_str(&f.summary.state) {
                Ok(state) => state,
                Err(e) => {
                    let event_type_error = EventTypesError::ParseString(e.into());
                    return Err(SyncthingError::EventTypeError(event_type_error))
                }
            },
            _ => {
                let parse_error_message = "error validating FolderSummary after filtering".to_string();
                return Err(SyncthingError::GenericMessage(parse_error_message))
            }
        };

        Ok(last_folder_state)
    }

    pub async fn fetch_events(&self) -> Result<Vec<SyncthingEvent>, SyncthingError> {
        let resp = self.client.get_events_since(&self.last_seen).await?;

        let json_str = resp.text().await?;

        let json_data:Vec<SyncthingEvent> = serde_json::from_str::<Vec<SyncthingEvent>>(&json_str)?;
    
        Ok(json_data)
    }

    pub fn filter_events(&self, all_events: &Vec<SyncthingEvent>, event_type: &EventTypes) -> Vec<SyncthingEvent> {
        let binding = event_type.clone();
        let event_type_str = binding.as_ref();
        Logger::log_debug_string(&format!("filtering for events of type: {}", event_type_str));
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