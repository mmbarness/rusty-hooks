use std::{path::{PathBuf}, sync::{Mutex, Arc}, time::Duration, fs};
use async_process::{Command, Output};
use futures::future::try_join_all;
use tokio::{sync::broadcast::Sender, task::JoinHandle};
use crate::{logger::{structs::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging}, errors::watcher_errors::{spawn_error::SpawnError, subscriber_error::SubscriptionError, thread_error::UnexpectedAnyhowError}};
use crate::scripts::structs::Script;
use crate::errors::watcher_errors::thread_error::ThreadError;
use crate::errors::script_errors::script_error::ScriptError;
use crate::utilities::traits::Utilities;
use super::structs::Runner;

impl Runner {
    pub fn new() -> Result<Self, ThreadError> {
        let spawn_channel = <Self as Utilities>::new_channel::<(PathBuf, Vec<Script>)>();
        let unsubscribe_broadcast_channel = <Self as Utilities>::new_channel::<PathBuf>();
        let script_runtime = <Self as Utilities>::new_runtime(4, &"script-runner".to_string())?;
        let script_runtime_arc = Arc::new(Mutex::new(script_runtime));
        Ok(Runner {
            runtime: script_runtime_arc,
            spawn_channel,
            unsubscribe_broadcast_channel
        })
    }

    pub async fn init(&self) -> Result<(), SpawnError> {
        let mut spawn_listener = self.spawn_channel.0.clone().subscribe();
        let runtime = self.runtime.clone();
        // listening for paths to run scripts on, sent over from the PathSubscriber
        loop {
            let (path, scripts) = spawn_listener.recv().await.map_err(ThreadError::RecvError)?;
            let path_string = path.to_str().unwrap_or("unable to pull string out of path buf");
            Logger::log_debug_string(&format!("new path to spawn scripts for: {}", path_string));
            let unsubscribe_clone = self.unsubscribe_broadcast_channel.0.clone();
            let runtime_lock = match runtime.lock() {
                Ok(lock) => lock,
                Err(e) => {
                    let poison_error_message = e.to_string();
                    let message = format!("unable to lock onto watched paths structure while receiving new path subscription: {}", poison_error_message);
                    Logger::log_error_string(&message);
                    return Ok(())
                }
            };

            let scripts_task:JoinHandle<Result<(), SpawnError>> = runtime_lock.spawn(async move {
                let script_processes:Vec<_> = scripts.iter().map(|script|{
                    Self::run(&script.file_path, &path, &script.run_delay)
                }).collect();
                let awaited_scripts = try_join_all(script_processes).await.map_err(|e| SpawnError::ScriptError(e.to_string()))?;
                for script in awaited_scripts {
                    match script.status.success() {
                        true => {
                            let stdout_str = String::from_utf8(script.stdout.clone()).unwrap_or("".to_string());
                            Logger::log_info_string(&format!("script execution successful"));
                            Logger::log_debug_string(&stdout_str);
                        },
                        false => {
                            let stderr_str = String::from_utf8(script.stderr.clone()).unwrap_or("".to_string());
                            Logger::log_error_string(&format!("error with a script"));
                            Logger::log_error_string(&stderr_str);
                        }
                    }
                }
                // will recursively attempt to unsubscribe more than once before erroring
                Self::rec_unsubscribe(unsubscribe_clone, path,5)?;
                Ok(())
            });
            scripts_task.await.map_err(ThreadError::JoinError)??
        };
    }

    fn rec_unsubscribe(unsub_channel: Sender<PathBuf>, path: PathBuf, num_retries: i8) -> Result<Option<usize>, SubscriptionError> {
        if num_retries <= 0  {
            return Err(SubscriptionError::new_unexpected_error(format!("unable to unsubscribe from path")))
        }
        let path_clone = path.clone();
        match unsub_channel.send(path_clone) {
            Ok(_) => {
                return Ok(None)
            },
            Err(e) => {
                let message = format!("error while attempting to unsubscribe from path, retrying...");
                Logger::log_error_string(&e.to_string());
                Logger::log_error_string(&message);
                return Self::rec_unsubscribe(unsub_channel, path, num_retries - 1)
            }
        }
    }

    async fn run(script_path: &PathBuf, target_path: &PathBuf, run_delay: &u8) -> Result<Output, ScriptError> {
        tokio::time::sleep(Duration::from_secs(run_delay.clone().into())).await;
        let path_string = script_path.to_str().ok_or(SpawnError::ArgError("failed to parse script path".to_string()))?;
        let target_path_string = target_path.to_str().ok_or(SpawnError::ArgError("failed to parse target path".to_string()))?;
        Logger::log_debug_string(&format!("script path is at: {}", path_string));
        Logger::log_debug_string(&format!("directory path is at: {}", target_path_string));
        match fs::try_exists(target_path) {
            Ok(_) => {
                Logger::log_debug_string(&"target path exists, attempting to run".to_string());
            },
            Err(e) => {
                return Err(ScriptError::IoError(e.into()))
            }
        }
        let canonicalized_target_path = target_path.canonicalize()?;
        Ok(Command::new(script_path)
            .arg(canonicalized_target_path.as_os_str())
            .output()
            .await?)
    }
}