use std::{sync::Arc, time::Duration};
use chrono::{DateTime, Utc};
use tokio::{sync::Mutex, time::sleep};
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

    pub fn time_to_break(&self) -> Result<bool, TimerError> {
        let now = chrono::prelude::Utc::now();
        let controller = self.controller.try_lock()?;
        let from_then_to_now_option = now.signed_duration_since(controller.1);
        // evaluate duration of time from shared state AKA original timestamp to now, returning a duration of 0 if the calculation yields something funky
        // that funky yield only happens if var now is somehow behind the original timestamp, which isnt possible unless theres weird concurrent operations happening, in which case waiting for this method to be called again is fine anyways
        Ok(from_then_to_now_option > controller.0)
    }

    pub async fn wait(&self) -> Result<(), TimerError> {
        loop {
            let should_break = self.time_to_break()?;
            if should_break { 
                break
            } else {
                sleep(Duration::from_millis(500)).await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use loom::thread;
    use super::Timer;

    #[tokio::test]
    async fn waits_longer_from_concurrent_controller_updates () {
        loom::model(|| {
            let timer = Timer::new(0);
            let controller_clone = timer.controller.clone();
    
            let handle = thread::spawn(move || {
                let now = chrono::prelude::Utc::now();
                let duration_to_add = chrono::Duration::hours(1);
                let one_hour_from_now = now.checked_add_signed(duration_to_add).unwrap();
                let mut controller_lock = controller_clone.try_lock().unwrap();
                controller_lock.1 = one_hour_from_now;
            });

            handle.join().unwrap();
            
            assert_eq!(timer.time_to_break().unwrap(), false)
        });
    }

    #[tokio::test]
    async fn can_handle_controller_updates() {
        let timer = Timer::new(1);
        let controller_clone = timer.controller.clone();
        let now = chrono::prelude::Utc::now();
        let mut controller_lock = controller_clone.try_lock().unwrap();
        controller_lock.1 = now;
    }

    #[tokio::test]
    async fn breaks_when_it_should() {
        let duration = tokio::time::Duration::new(1, 1);
        let timer = Timer::new(1);
        tokio::time::sleep(duration).await;
        assert_eq!(timer.time_to_break().unwrap(), true)
    }

    #[tokio::test]
    async fn doesnt_break_when_it_should() {
        let duration = tokio::time::Duration::new(0, 1);
        let timer = Timer::new(2);
        tokio::time::sleep(duration).await;
        assert_eq!(timer.time_to_break().unwrap(), false)
    }

    #[tokio::test]
    async fn waits_long_enough () {
        let before = chrono::prelude::Utc::now();
        let chrono_duration = chrono::Duration::seconds(1);
        let timer = Timer::new(2);
        timer.wait().await.unwrap();
        let now = chrono::prelude::Utc::now();
        let duration_since_before_waiting = now.signed_duration_since(before);
        let has_been_long_enough = duration_since_before_waiting >= chrono_duration;
        assert_eq!(has_been_long_enough, true)
    }
}