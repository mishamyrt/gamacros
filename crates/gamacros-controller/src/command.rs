use crate::types::ControllerId;

/// Internal commands sent to the runtime thread.
pub(crate) enum Command {
    Rumble { id: ControllerId, low: u16, high: u16, ms: u32 },
    StopRumble { id: ControllerId },
}


