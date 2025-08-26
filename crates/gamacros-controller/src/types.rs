/// Unique identifier of a controller or joystick device.
pub type ControllerId = u32;

/// Logical controller buttons supported by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    A,
    B,
    X,
    Y,
    Back,
    Guide,
    Start,
    LeftStick,
    RightStick,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Controller meta information that remains stable across events.
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    pub id: ControllerId,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
}


