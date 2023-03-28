use std::num::ParseIntError;
use strum::ParseError;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum ConfigError{
    #[error("Unable to find env var: `{0}`")]
    ArgError(String),
    #[error("Error while parsing env vars: `{0}`")]
    ParseError(String),
    #[error("Error with strum: `{0}`")]
    StrumParseError(#[from] ParseError),
    #[error("Error with strum: `{0}`")]
    ParseIntError(#[from] ParseIntError),
}