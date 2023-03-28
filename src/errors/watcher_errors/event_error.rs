use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventError {
    #[error("error while watching file system events: `{0}`")]
    NotifyError(#[from] notify::Error),
}
