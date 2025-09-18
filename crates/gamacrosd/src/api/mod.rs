mod unix_sock;

use std::thread::JoinHandle;

pub use unix_sock::{UnixSocket};

use bitcode::{Decode, Encode};
use crossbeam_channel::Sender;
use gamacros_gamepad::ControllerId;
use thiserror::Error;

/// Error type for api operations.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("failed to send event")]
    IoError(#[from] std::io::Error),
}

/// Convenient result alias for api operations.
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// gamacrosd api control command.
#[derive(Encode, Decode)]
pub enum Command {
    Rumble { id: Option<ControllerId>, ms: u32 },
}

/// gamacrosd api events transport.
/// listener that can receive api commands from the outer world,
/// and sender that can send api commands from the outer world to the gamacrosd.
pub trait ApiTransport {
    fn listen_events(&self, tx: Sender<Command>) -> ApiResult<JoinHandle<()>>;
    fn send_event(&self, event: Command) -> ApiResult<()>;
}
