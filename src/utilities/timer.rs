use std::{sync::Arc, time::Duration};
use chrono::{DateTime, Utc};
use tokio::{sync::Mutex, time::{sleep}};

use crate::errors::watcher_errors::timer_error::TimerError;

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
        let controller = self.controller.try_lock()?;
        let from_then_to_now_option = now.signed_duration_since(controller.1);
        // evaluate duration of time from shared state AKA original timestamp to now, returning a duration of 0 if the calculation yields something funky
        // that funky yield only happens if var now is somehow behind the original timestamp, which isnt possible unless theres weird concurrent operations happening, in which case waiting for this method to be called again is fine anyways
        Ok(from_then_to_now_option > controller.0)
    }

    pub async fn wait(&self) -> Result<(), TimerError> {
        loop {
            let should_break = self.time_to_break().await?;
            if should_break { 
                break
            } else {
                sleep(Duration::from_millis(500)).await;
            }
        }
        Ok(())
    }
}