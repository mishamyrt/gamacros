mod profile;
mod profile_parse;
mod v1;
mod profile_watcher;
mod workspace;

use thiserror::Error;

use gamacros_bit_mask::Bitmask;
use gamacros_gamepad::Button;

pub use profile_watcher::{ProfileWatcher, ProfileEvent};

pub use profile_parse::parse_profile;
pub use profile::{
    Profile, ButtonAction, ButtonRule, ControllerSettings, StickRules, ArrowsParams,
    Axis, MouseParams, ScrollParams, StepperParams, StickMode, StickSide,
};
// pub use profile::resolve_profile;
pub use workspace::Workspace;

/// A macOS application bundle ID.
pub type BundleId = Box<str>;

/// A controller ID.
/// Vendor ID and product ID.
pub type ControllerId = (u16, u16);

/// A chord of buttons.
pub type ButtonChord = Bitmask<Button>;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("profile error: {0}")]
    ProfileError(#[from] profile::ProfileError),
    #[error("watcher error: {0}")]
    WatcherError(#[from] profile_watcher::WatcherError),

    #[error("environment variable not set: {0}")]
    EnvVarNotSet(String),
    #[error("path is not a directory: {0}")]
    PathIsNotDirectory(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
