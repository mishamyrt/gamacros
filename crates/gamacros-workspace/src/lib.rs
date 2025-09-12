mod workspace;
mod parse;
mod resolve;
mod v1;
mod watcher;

use thiserror::Error;

use gamacros_bit_mask::Bitmask;
use gamacros_gamepad::Button;

pub use watcher::{WorkspaceWatcher, WorkspaceEvent};

pub use parse::parse_profile;
pub use workspace::{
    Workspace, ButtonAction, ButtonRule, ControllerSettings, StickRules,
    ArrowsParams, Axis, MouseParams, ScrollParams, StepperParams, StickMode,
    StickSide,
};
pub use resolve::resolve_profile;

/// A macOS application bundle ID.
pub type BundleId = Box<str>;

/// A controller ID.
/// Vendor ID and product ID.
pub type ControllerId = (u16, u16);

/// A chord of buttons.
pub type ButtonChord = Bitmask<Button>;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("yaml deserialize error: {0}")]
    YamlDeserializeError(#[from] serde_yaml::Error),
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("v1 profile error: {0}")]
    V1ProfileError(#[from] v1::Error),

    #[error("environment variable not set: {0}")]
    EnvVarNotSet(String),
    #[error("current directory not set")]
    CurrentDirNotSet,
    #[error("profile not found: {0}")]
    ProfileNotFound(String),
    #[error("path error: {0}")]
    PathError(#[from] std::io::Error),
}
