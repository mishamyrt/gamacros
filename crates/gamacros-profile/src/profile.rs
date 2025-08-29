use core::str;
use std::collections::HashMap;

use gamacros_bit_mask::Bitmask;
use gamacros_controller::Button;
use gamacros_keypress::KeyCombo;

use crate::ProfileError;

/// A macOS application bundle ID.
pub type BundleId = Box<str>;

/// A chord of buttons.
pub type ButtonChord = Bitmask<Button>;

/// A binding of stick sides to rules.
pub type StickBinding = HashMap<StickSide, StickMode>;

/// A set of rules to handle button presses for an app.
pub type ButtonRules = HashMap<ButtonChord, ButtonRule>;

/// A set of rules to handle stick movements for an app.
pub type StickRules = HashMap<StickSide, StickMode>;

/// A set of rules to handle controller settings for an app.
pub type RulesByApp = HashMap<BundleId, AppRules>;

/// A set of rules to handle controller settings for an app.
#[derive(Debug, Clone)]
pub struct AppRules {
    pub buttons: ButtonRules,
    pub sticks: StickRules,
}

/// Profile is a collection of rules and settings for controllers and applications.
#[derive(Debug, Clone)]
pub struct Profile {
    /// Controller settings.
    pub controllers: Vec<ControllerParams>,
    /// App rules. Bundle ID -> app rules.
    pub rules: RulesByApp,
}

/// A phase of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonPhase {
    Pressed,
    Released,
}

/// A controller settings.
#[derive(Debug, Clone)]
pub struct ControllerParams {
    pub vid: u16,
    pub pid: u16,
    pub remap: HashMap<Button, Button>,
}

/// A rule for a gamepad button.
#[derive(Debug, Clone)]
pub struct ButtonRule {
    pub action: KeyCombo,
    pub when: ButtonPhase,
    pub vibrate: Option<u16>,
}

/// A side of a stick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StickSide {
    Left,
    Right,
}

impl StickSide {
    pub fn parse(raw: &str) -> Result<StickSide, ProfileError> {
        Ok(match raw.to_lowercase().as_str() {
            "left" => StickSide::Left,
            "right" => StickSide::Right,
            other => return Err(ProfileError::InvalidStickSide(other.to_string())),
        })
    }
}

/// An axis of a stick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
}

/// A mode of a gamepad stick.
#[derive(Debug, Clone)]
pub enum StickMode {
    Arrows(ArrowsParams),
    Volume(StepperParams),
    Brightness(StepperParams),
    MouseMove(MouseParams),
    Scroll(ScrollParams),
}

/// Parameters for the arrows mode.
#[derive(Debug, Clone)]
pub struct ArrowsParams {
    pub deadzone: f32,
    pub repeat_delay_ms: u64,
    pub repeat_interval_ms: u64,
    pub invert_x: bool,
    pub invert_y: bool,
}

/// Parameters for the volume/brightness modes.
#[derive(Debug, Clone)]
pub struct StepperParams {
    pub axis: Axis,
    pub deadzone: f32,
    pub min_interval_ms: u64,
    pub max_interval_ms: u64,
    pub invert: bool,
}

/// Parameters for the mouse move mode.
#[derive(Debug, Clone)]
pub struct MouseParams {
    pub deadzone: f32,
    pub max_speed_px_s: f32,
    pub gamma: f32,
    pub invert_x: bool,
    pub invert_y: bool,
}

/// Parameters for the scroll mode.
#[derive(Debug, Clone)]
pub struct ScrollParams {
    pub deadzone: f32,
    pub speed_lines_s: f32,
    pub horizontal: bool,
    pub invert_x: bool,
    pub invert_y: bool,
}
