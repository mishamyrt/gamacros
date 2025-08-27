mod command;
mod events;
mod handle;
mod manager;
mod runtime;
mod types;

use thiserror::Error;

pub use crate::events::{ControllerEvent, EventReceiver};
pub use crate::handle::ControllerHandle;
pub use crate::manager::ControllerManager;
pub use crate::types::{Button, ControllerId, ControllerInfo};

/// Error type for controller management operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to initialize the backend (SDL2 or subsystems).
    #[error("Backend init failed: {0}")]
    BackendInit(String),
    /// Requested controller was not found.
    #[error("Controller not found: {0}")]
    NotFound(u32),
    /// Operation is not supported on the current device/backend.
    #[error("Operation unsupported")]
    Unsupported,
    /// A generic backend error.
    #[error("Backend error: {0}")]
    Backend(String),
}

/// Convenient result alias for controller operations.
pub type Result<T> = std::result::Result<T, Error>;
