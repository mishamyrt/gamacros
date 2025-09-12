mod strings;
mod parse;
mod profile;
mod selector;

use thiserror::Error;

pub use profile::ProfileV1;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid trigger: {0}")]
    InvalidTrigger(String),
    #[error("invalid actions for {0}")]
    InvalidActions(String),
    #[error("invalid id: {0} -> {1}")]
    InvalidId(String, String),
    #[error("invalid button: {0}")]
    InvalidButton(String),
    #[error("invalid stick: {0}")]
    InvalidStick(String),
    #[error("invalid stick side: {0}")]
    InvalidStickSide(String),
    #[error("invalid axis: {0}")]
    InvalidAxis(String),
    #[error("key parse error: {0}")]
    KeyParse(String),
    #[error("no profile matches path \"{0}\"")]
    ProfileNotFound(String),
    #[error("selector error: {0}")]
    BadSelector(#[from] selector::SelectorError),
}
