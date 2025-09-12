mod profile;
mod profile_common;
mod profile_v1;
mod selector;
mod resolve;

use thiserror::Error;

pub use profile_common::parse_profile;
pub use profile::*;
pub use resolve::resolve_profile;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u8),
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
    KeyParseError(String),
    #[error("no profile matches path \"{0}\"")]
    ProfileNotFound(String),

    #[error("environment variable not set: {0}")]
    EnvVarNotSet(String),
    #[error("current directory not set")]
    CurrentDirNotSet,
    #[error("invalid selector: {0}")]
    InvalidSelector(#[from] selector::SelectorError),
}
