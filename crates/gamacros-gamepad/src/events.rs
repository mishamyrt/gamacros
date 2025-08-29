use crossbeam_channel::Receiver;

use crate::types::{Button, ControllerId, ControllerInfo, Axis};

/// Events emitted by the manager about controller lifecycle and input.
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    /// A controller or joystick has been connected and enumerated.
    Connected(ControllerInfo),
    /// A previously connected controller has been disconnected.
    Disconnected(ControllerId),
    /// A logical controller button was pressed.
    ButtonPressed { id: ControllerId, button: Button },
    /// A logical controller button was released.
    ButtonReleased { id: ControllerId, button: Button },
    /// An analog axis moved; value is normalized to [-1.0, 1.0].
    AxisMotion { id: ControllerId, axis: Axis, value: f32 },
}

/// Receiving end for controller events subscription.
pub type EventReceiver = Receiver<ControllerEvent>;
