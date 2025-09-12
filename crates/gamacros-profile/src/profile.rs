use std::sync::Arc;
use core::str;
use ahash::{AHashMap, AHashSet};

use gamacros_bit_mask::Bitmask;
use gamacros_gamepad::Button;
use gamacros_control::KeyCombo;

use crate::ProfileError;

pub const DEFAULT_SHELL: &str = "/bin/sh";

/// A macOS application bundle ID.
pub type BundleId = Box<str>;

/// A controller ID.
/// Vendor ID and product ID.
pub type ControllerId = (u16, u16);

/// A chord of buttons.
pub type ButtonChord = Bitmask<Button>;

/// A binding of stick sides to rules.
pub type StickBinding = AHashMap<StickSide, StickMode>;

/// A set of rules to handle button presses for an app.
pub type ButtonRules = AHashMap<ButtonChord, ButtonRule>;

/// A set of rules to handle stick movements for an app.
pub type StickRules = AHashMap<StickSide, StickMode>;

/// A set of rules to handle controller settings for an app.
#[derive(Debug, Clone, Default)]
pub struct AppRules {
    pub buttons: ButtonRules,
    pub sticks: StickRules,
}

/// Controller parameters.
#[derive(Debug, Clone, Default)]
pub struct ControllerSettings {
    pub mapping: AHashMap<Button, Button>,
}

impl ControllerSettings {
    pub fn new(mapping: AHashMap<Button, Button>) -> Self {
        Self { mapping }
    }
}

/// A set of rules to handle app settings for an app.
pub type RuleMap = AHashMap<BundleId, AppRules>;

/// A set of rules to handle app settings for an app.
pub type ControllerSettingsMap = AHashMap<ControllerId, ControllerSettings>;

/// A set of rules to handle app settings for an app.
pub type Blacklist = AHashSet<String>;

/// Profile is a collection of rules and settings for controllers and applications.
#[derive(Debug, Clone)]
pub struct Profile {
    /// Controller settings.
    pub controllers: ControllerSettingsMap,
    /// Blacklist apps.
    pub blacklist: AHashSet<String>,
    /// App rules.
    pub rules: RuleMap,
    /// Shell to run for shell actions.
    pub shell: Box<str>,
}

/// A action for a gamepad button.
#[derive(Debug, Clone)]
pub enum ButtonAction {
    Keystroke(Arc<KeyCombo>),
    Macros(Arc<Vec<KeyCombo>>),
    Shell(String),
}

/// A rule for a gamepad button.
#[derive(Debug, Clone)]
pub struct ButtonRule {
    pub action: ButtonAction,
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
