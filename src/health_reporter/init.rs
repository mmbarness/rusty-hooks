use crate::{errors::shared_errors::thread_errors::ThreadError, utilities::traits::Utilities};
use log::info;
use tokio::{task::JoinHandle, time::sleep};

#[derive(Debug)]
pub struct HealthReporter {}

impl Utilities for HealthReporter {}

impl HealthReporter {
    pub fn new() -> Result<Self, ThreadError> {
        Ok(HealthReporter {})
    }

    pub fn begin_reporting(&self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                info!(target: "health_report", "rusty hooks is healthy");
                sleep(tokio::time::Duration::from_secs(30)).await;
            }
        })
    }
}
