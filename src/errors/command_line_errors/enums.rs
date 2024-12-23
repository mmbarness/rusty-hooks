use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandLineError {
    #[error("error with cli argument provided for script configuration file: `{0}`")]
    ScriptConfigError(String),
    #[error("io error while parsing command line args: `{0}`")]
    IoError(#[from] std::io::Error),
}

