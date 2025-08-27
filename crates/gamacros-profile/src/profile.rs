use std::collections::HashMap;

use gamacros_bit_mask::Bitmask;
use gamacros_controller::Button;
use gamacros_keypress::KeyCombo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerPhase {
    Pressed,
    Released,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub app_rules: HashMap<Box<str>, Vec<Rule>>, // bundle_id -> rules
    pub device_remaps: Vec<DeviceRemap>,
}

#[derive(Debug, Clone)]
pub struct DeviceRemap {
    pub vid: u16,
    pub pid: u16,
    pub mapping: HashMap<Button, Button>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub trigger: Trigger,
    pub action: Action,
    pub vibrate: Option<u16>,
    pub when: TriggerPhase,
}

pub type ButtonChord = Bitmask<Button>;

#[derive(Debug, Clone)]
pub enum Trigger {
    Chord(ButtonChord),
    // TODO: add axis
}

#[derive(Debug, Clone)]
pub enum Action {
    Key(KeyCombo),
    None,
}
