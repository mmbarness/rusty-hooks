use std::{path::{PathBuf}, sync::{Mutex, Arc}, time::Duration};
use async_process::{Child, Command, Output};
use futures::future::try_join_all;
use tokio::sync::broadcast::{Receiver, Sender};
use log::info;
use crate::logger::{r#struct::Logger, error::ErrorLogging, info::InfoLogging, debug::DebugLogging};
use super::{watcher_errors::{ script_error::ScriptError}, watcher_scripts::{ Script}};

pub struct Runner {
    pub runtime: Arc<Mutex<tokio::runtime::Runtime>>,
    pub spawn_channel: (Sender<(PathBuf, Vec<Script>)>, Receiver<(PathBuf, Vec<Script>)>)
}

impl Runner {
    pub fn new() -> Self {
        let spawn_channel = tokio::sync::broadcast::channel::<(PathBuf, Vec<Script>)>(16);
        let script_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("script-runner")
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        let script_runtime_arc = Arc::new(Mutex::new(script_runtime));
        Runner {
            runtime: script_runtime_arc,
            spawn_channel,
        }
    }

    pub async fn init(&self, unsubscribe: Sender<PathBuf>) {
        let mut spawn_listener = self.spawn_channel.0.clone().subscribe();
        let runtime = self.runtime.clone();
        // listening for paths to run scripts on, sent over from the PathSubscriber
        while let Ok((path, scripts)) = spawn_listener.recv().await {
            let unsubscribe_clone = unsubscribe.clone();
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
                    Self::run(&script.file_name, path.clone())
                }).collect();
                let awaited_scripts = match try_join_all(script_processes).await {
                    Ok(vec) => vec,
                    Err(e) => {
                        Logger::log_error_string(&format!("error while executing script: {}", e.to_string()));
                        return ();
                    }
                };
                for script in awaited_scripts {
                    Logger::log_info_string(&format!("printing script stdout...: {:?}", script.stdout));
                    match script.status.success() {
                        true => {},
                        false => {
                            Logger::log_error_string(&format!("successfully ran script, printing stderr...: {:?}", script.stderr))
                        }
                    }
                }
                let path_clone = path.clone();
                let path_display = path_clone.display();
                let unsubscribe_success_message = &format!("successfully unsubscribed from path: {}", path_display);
                match unsubscribe_clone.send(path) {
                    Ok(_) => {
                        Logger::log_info_string(unsubscribe_success_message)
                    },
                    Err(e) => {
                        let message = format!("error while attempting to unsubscribe from path, retrying...");
                        Logger::log_error_string(&message);
                        Logger::log_debug_string(&e.to_string());
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

    async fn run(script_path: &String, path: PathBuf) -> Result<Output, ScriptError> {
        Ok(Command::new("sh")
            .arg("-C")
            .arg(script_path.to_string())
            .output()
            .await?)
    }

}