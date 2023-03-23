use std::{sync::Arc, time::Duration};
use chrono::{DateTime, Utc};
use tokio::{sync::Mutex, time::{sleep}};

use crate::logger::{r#struct::Logger, error::ErrorLogging};

use super::errors::TimerError;

pub type TimerController = Arc<Mutex<(chrono::Duration,DateTime<Utc>)>>;
pub struct Timer {
    pub controller: TimerController
}

impl Timer {
    pub fn new(wait_duration: i64) -> Self {
        let duration = chrono::Duration::seconds(wait_duration);
        let waiting_from = chrono::prelude::Utc::now();
        Timer {
            controller: Arc::new(Mutex::new((duration, waiting_from)))
        }
    }

    pub async fn time_to_break(&self) -> Result<bool, TimerError> {
        let now = chrono::prelude::Utc::now();
        let controller = self.controller.lock().await;
        let from_then_to_now_option = now.signed_duration_since(controller.1);
        // evaluate duration of time from shared state AKA original timestamp to now, returning a duration of 0 if the calculation yields something funky
        // that funky yield only happens if var now is somehow behind the original timestamp, which isnt possible unless theres weird concurrent operations happening, in which case waiting for this method to be called again is fine anyways
        Ok(from_then_to_now_option > controller.0)
    }

    pub async fn wait<'a>(&self) -> Result<(), TimerError<'a>> {
        // receive a (duration, timestamp_to_depend_on) arc<mutex> tuple
        loop {
            let should_break = match self.time_to_break().await {
                Ok(should) => should,
                Err(e) => {
                    Logger::log_error_string(&e.to_string());
                    false
                }
            };
            if should_break { 
                break
            } else {
                sleep(Duration::from_millis(500)).await;
            }
        }
        return Ok(())
    }
}