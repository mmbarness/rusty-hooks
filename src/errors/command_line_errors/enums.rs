use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandLineError {
    #[error("error with cli argument provided for script configuration file: `{0}`")]
    ScriptConfigError(String)
}