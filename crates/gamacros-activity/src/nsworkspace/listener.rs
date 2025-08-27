use std::sync::mpsc;

use crate::monitor::Event;

use super::{app_delegate::AppDelegate, NSWorkspaceError};

pub(crate) fn start_nsworkspace_listener(
    tx: mpsc::Sender<Event>,
) -> Result<(), NSWorkspaceError> {
    AppDelegate::new(tx)?.start_listening();
    Ok(())
}
