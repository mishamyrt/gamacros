use std::sync::Arc;
use std::time::Duration;

use crate::command::Command;
use crate::error::{Error, Result};
use crate::manager::Inner;
use crate::types::ControllerId;

/// A handle to a specific controller, providing operations such as rumble.
#[derive(Clone)]
pub struct ControllerHandle {
    pub(crate) id: ControllerId,
    pub(crate) inner: Arc<Inner>,
}

impl ControllerHandle {
    /// Returns the unique identifier of the underlying controller.
    pub fn id(&self) -> ControllerId {
        self.id
    }

    /// Triggers the controller rumble, if supported by the device.
    /// - `low_freq` and `high_freq` are normalized in [0.0, 1.0]
    /// - `duration` specifies how long the rumble should play
    pub fn rumble(&self, low_freq: f32, high_freq: f32, duration: Duration) -> Result<()> {
        let low = (low_freq.clamp(0.0, 1.0) * 65535.0).round() as u16;
        let high = (high_freq.clamp(0.0, 1.0) * 65535.0).round() as u16;
        let ms = duration.as_millis().min(u32::MAX as u128) as u32;
        self.inner
            .cmd_tx
            .send(Command::Rumble { id: self.id, low, high, ms })
            .map_err(|e| Error::Backend(format!("{e}")))
    }

    /// Stops the controller rumble if it is currently active.
    pub fn stop_rumble(&self) -> Result<()> {
        self.inner
            .cmd_tx
            .send(Command::StopRumble { id: self.id })
            .map_err(|e| Error::Backend(format!("{e}")))
    }
}


