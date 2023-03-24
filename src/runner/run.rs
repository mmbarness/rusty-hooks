use std::{path::{PathBuf}, sync::{Mutex, Arc}, time::Duration};
use async_process::{Command, Output};
use futures::future::try_join_all;
use crate::{logger::{r#struct::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging}, errors::watcher_errors::thread_error::ThreadError};
use crate::errors::watcher_errors::script_error::ScriptError;
use crate::utilities::traits::Utilities;
use crate::watcher::watcher_scripts::Script;
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

    pub async fn init(&self) {
        let mut spawn_listener = self.spawn_channel.0.clone().subscribe();
        let runtime = self.runtime.clone();
        // listening for paths to run scripts on, sent over from the PathSubscriber
        while let Ok((path, scripts)) = spawn_listener.recv().await {
            let path_string = path.to_str().unwrap_or("unable to pull string out of path buf");
            Logger::log_debug_string(&format!("new path to spawn scripts for: {}", path_string));
            let unsubscribe_clone = self.unsubscribe_broadcast_channel.0.clone();
            let runtime_lock = match runtime.lock() {
                Ok(lock) => lock,
                Err(e) => {
                    let poison_error_message = e.to_string();
                    let message = format!("unable to lock onto watched paths structure whilst receiving new path subscription: {}", poison_error_message);
                    Logger::log_error_string(&message);
                    return ()
                }
            };
            runtime_lock.spawn(async move {
                tokio::time::sleep_until(tokio::time::Instant::now() + Duration::from_secs(10)).await;
                let script_processes:Vec<_> = scripts.iter().map(|script|{
                    Self::run(&script.file_path, &path)
                }).collect();
                let awaited_scripts = match try_join_all(script_processes).await {
                    Ok(vec) => vec,
                    Err(e) => {
                        Logger::log_error_string(&format!("error while executing script: {}", e.to_string()));
                        return ();
                    }
                };
                for script in awaited_scripts {
                    match script.status.success() {
                        true => {
                            Logger::log_info_string(&format!("printing script stdout...: {:?}", String::from_utf8(script.stdout)));
                        },
                        false => {
                            Logger::log_error_string(&format!("one or several of the scripts returned a stderr...: {:?}", String::from_utf8(script.stderr)))
                        }
                    }
                }
                let path_clone = path.clone();
                let path_display = path_clone.display();
                let unsubscribe_success_message = &format!("successfully unsubscribed from path: {}", path_display);
                match unsubscribe_clone.send(path) {
                    Ok(_) => {},
                    Err(e) => {
                        let message = format!("retrying...");
                        Logger::log_error_string(&e.to_string());
                        Logger::log_error_string(&message);
                        match unsubscribe_clone.send(path_clone) {
                            Ok(_) => {
                                Logger::log_info_string(unsubscribe_success_message)
                            },
                            Err(e) => {
                                let message = format!("error while attempting to unsubscribe from path, retrying...");
                                Logger::log_error_string(&message);
                                Logger::log_debug_string(&e.to_string())
                                // need to implement a way to panic core process and probably a way to reset the path subscriber in the event it becomes unreachable
                            }
                        }
                    }
                }
            });
        }
    }

    async fn run(script_path: &PathBuf, target_path: &PathBuf) -> Result<Output, ScriptError> {
        let path_string = match script_path.to_str() {
            Some(s) => s,
            None => "uh",
        };
        Logger::log_debug_string(&format!("path is at: {}", path_string));
        Ok(Command::new(script_path)
            .arg(target_path)
            .output()
            .await?)
    }

}