use crate::{
    errors::shared_errors::thread_errors::ThreadError,
    utilities::traits::Utilities
};
use log::info;
use tokio::{task::JoinHandle, time::sleep};

#[derive(Debug)]
pub struct HealthReporter {
    pub runtime: tokio::runtime::Runtime,
}

impl Utilities for HealthReporter {}

impl HealthReporter {
    pub fn new() -> Result<Self, ThreadError> {
        let script_runtime = <Self as Utilities>::new_runtime(1, &"health_check".to_string())?;
        Ok(HealthReporter {
            runtime: script_runtime,
        })
    }

    pub fn begin_reporting(&self) -> JoinHandle<()> {
        self.runtime.spawn(async move {
            loop {
                info!("rusty hooks is healthy");
                sleep(tokio::time::Duration::from_secs(30)).await;
            }
        })
    }
}
