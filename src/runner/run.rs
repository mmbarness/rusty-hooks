use super::structs::Runner;
use crate::errors::script_errors::script_error::ScriptError;
use crate::errors::shared_errors::thread_errors::{ThreadError, UnexpectedAnyhowError};
use crate::errors::watcher_errors::{spawn_error::SpawnError, subscriber_error::SubscriptionError};
use crate::scripts::structs::Script;
use crate::utilities::traits::Utilities;
use async_process::{Command, Output};
use futures::future::try_join_all;
use log::{debug, error, info};
use std::{fs, path::PathBuf, time::Duration};
use tokio::{sync::broadcast::Sender, task::JoinHandle};

impl Runner {
    pub fn new() -> Result<Self, ThreadError> {
        let spawn_channel = <Self as Utilities>::new_channel::<(PathBuf, Vec<Script>)>();
        let unsubscribe_broadcast_channel = <Self as Utilities>::new_channel::<PathBuf>();
        let script_runtime = <Self as Utilities>::new_runtime(4, &"script-runner".to_string())?;
        Ok(Runner {
            runtime: script_runtime,
            spawn_channel,
            unsubscribe_broadcast_channel,
        })
    }

    pub async fn init(&self) -> Result<(), SpawnError> {
        let mut spawn_listener = self.spawn_channel.0.clone().subscribe();
        // listening for paths to run scripts on, sent over from the PathSubscriber
        loop {
            let (path, scripts) = spawn_listener
                .recv()
                .await
                .map_err(ThreadError::RecvError)?;
            let path_string = path
                .to_str()
                .unwrap_or("unable to pull string out of path buf");
            debug!("new path to spawn scripts for: {}", path_string);
            let unsubscribe_clone = self.unsubscribe_broadcast_channel.0.clone();

            let scripts_task: JoinHandle<Result<(), SpawnError>> = self.runtime.spawn(async move {
                let script_processes: Vec<_> = scripts
                    .iter()
                    .map(|script| Self::run(&script.file_path, &path, &script.run_delay))
                    .collect();
                let awaited_scripts = try_join_all(script_processes)
                    .await
                    .map_err(|e| SpawnError::ScriptError(e.to_string()))?;
                Self::log_script_output(awaited_scripts);
                Self::rec_unsubscribe(unsubscribe_clone, path, 5)?;
                Ok(())
            });
            scripts_task.await.map_err(ThreadError::JoinError)??
        }
    }

    /// Recursively attempts to unsubscribe as many times as indicated by num_retries
    fn rec_unsubscribe(
        unsub_channel: Sender<PathBuf>,
        path: PathBuf,
        num_retries: i8,
    ) -> Result<Option<usize>, SubscriptionError> {
        if num_retries <= 0 {
            return Err(SubscriptionError::new_unexpected_error(format!(
                "unable to unsubscribe from path"
            )));
        }
        let path_clone = path.clone();
        match unsub_channel.send(path_clone) {
            Ok(_) => return Ok(None),
            Err(e) => {
                error!("{:?}", e);
                error!("error while attempting to unsubscribe from path, retrying...");
                return Self::rec_unsubscribe(unsub_channel, path, num_retries - 1);
            }
        }
    }

    async fn run(
        script_path: &PathBuf,
        target_path: &PathBuf,
        run_delay: &u8,
    ) -> Result<Output, ScriptError> {
        tokio::time::sleep(Duration::from_secs(run_delay.clone().into())).await;
        let script_path_string = script_path.to_str().ok_or(SpawnError::ArgError(
            "failed to parse script path".to_string(),
        ))?;
        let target_path_string = target_path.to_str().ok_or(SpawnError::ArgError(
            "failed to parse target path".to_string(),
        ))?;
        debug!("script path is at: {}", script_path_string);
        debug!("directory path is at: {}", target_path_string);
        match fs::exists(target_path) {
            Ok(_) => {
                debug!("target path exists, attempting to run");
            }
            Err(e) => return Err(ScriptError::IoError(e.into())),
        }
        let canonicalized_target_path = target_path.canonicalize()?;

        Ok(Command::new(script_path)
            .arg(canonicalized_target_path.as_os_str())
            .output()
            .await?)
    }

    fn log_script_output(awaited_scripts: Vec<Output>) {
        for script in awaited_scripts {
            match script.status.success() {
                true => {
                    let stdout_str =
                        String::from_utf8(script.stdout.clone()).unwrap_or("".to_string());
                    stdout_str
                        .split("\n")
                        .filter(|l| l != &"")
                        .for_each(|line| debug!(target: "script_output", "{:?}", line));
                    info!("script execution successful");
                }
                false => {
                    let stderr_str =
                        String::from_utf8(script.stderr.clone()).unwrap_or("".to_string());
                    error!("error with a script");
                    error!(target: "script_output", "{}", stderr_str);
                }
            }
        }
    }
}
