use ahash::AHashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileV1 {
    pub version: u8,
    #[serde(default)]
    pub controllers: Vec<ProfileV1ControllerSettings>,
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub groups: AHashMap<String, Vec<Box<str>>>,
    #[serde(default)]
    pub rules: AHashMap<Box<str>, ProfileV1App>, // bundle_id -> app mapping
    #[serde(default)]
    pub shell: Option<Box<str>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1App {
    #[serde(default)]
    pub buttons: AHashMap<String, ProfileV1ButtonRule>, // chord -> button rule
    #[serde(default)]
    pub sticks: AHashMap<String, ProfileV1Stick>, // side -> stick rules
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1ButtonRule {
    #[serde(default)]
    pub vibrate: Option<u16>,
    #[serde(default)]
    pub keystroke: Option<String>,
    #[serde(default)]
    pub macros: Option<Vec<String>>,
    #[serde(default)]
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1ControllerSettings {
    pub vid: u16,
    pub pid: u16,
    #[serde(default)]
    pub remap: AHashMap<String, String>, // button -> button
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1Stick {
    pub mode: String, // arrows | volume | brightness | scroll | mouse_move
    #[serde(default)]
    pub deadzone: Option<f32>,
    // arrows
    #[serde(default)]
    pub repeat_delay_ms: Option<u64>,
    #[serde(default)]
    pub repeat_interval_ms: Option<u64>,
    #[serde(default)]
    pub invert_x: Option<bool>,
    #[serde(default)]
    pub invert_y: Option<bool>,
    // stepper (volume/brightness)
    #[serde(default)]
    pub axis: Option<String>, // x | y
    #[serde(default)]
    pub invert: Option<bool>,
    #[serde(default)]
    pub min_interval_ms: Option<u64>,
    #[serde(default)]
    pub max_interval_ms: Option<u64>,
    // mouse
    #[serde(default)]
    pub max_speed_px_s: Option<f32>,
    #[serde(default)]
    pub gamma: Option<f32>,
    // scroll
    #[serde(default)]
    pub speed_lines_s: Option<f32>,
    #[serde(default)]
    pub horizontal: Option<bool>,
}
