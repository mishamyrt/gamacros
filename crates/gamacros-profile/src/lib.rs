use std::collections::{HashMap, HashSet};

use gamacros_controller::Button;
use gamacros_keypress::KeyCombo;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("yaml: {0}")]
    Yaml(String),
    #[error("invalid trigger: {0}")]
    InvalidTrigger(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerPhase {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    pub version: u8,
    #[serde(default)]
    pub gamepads: Vec<RawGamepadRemap>,
    #[serde(default)]
    pub apps: HashMap<String, Vec<RawRule>>, // bundle_id -> rules
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGamepadRemap {
    pub vid: serde_yaml::Value,
    pub pid: serde_yaml::Value,
    #[serde(default)]
    pub mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    pub trigger: serde_yaml::Value,
    pub action: serde_yaml::Value,
    #[serde(default)]
    pub vibrate: Option<u16>,
    #[serde(default)]
    pub when: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub app_rules: HashMap<String, Vec<Rule>>, // bundle_id -> rules
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
    pub trigger: TriggerSpec,
    pub action: ActionSpec,
    pub vibrate: Option<u16>,
    pub when: TriggerPhase,
}

#[derive(Debug, Clone)]
pub enum TriggerSpec {
    Chord(HashSet<Button>),
    Dpad(DpadMapping),
}

#[derive(Debug, Clone)]
pub struct DpadMapping {
    pub up: Option<ActionSpec>,
    pub down: Option<ActionSpec>,
    pub left: Option<ActionSpec>,
    pub right: Option<ActionSpec>,
}

#[derive(Debug, Clone)]
pub enum ActionSpec {
    Key(KeyCombo),
    None,
}

impl Profile {
    pub fn empty() -> Self {
        Self {
            app_rules: HashMap::new(),
            device_remaps: Vec::new(),
        }
    }

    pub fn from_yaml_str(input: &str) -> Result<Self, ProfileError> {
        let raw: RawProfile =
            serde_yaml::from_str(input).map_err(|e| ProfileError::Yaml(e.to_string()))?;
        if raw.version != 1 {
            return Err(ProfileError::Yaml("unsupported version".into()));
        }

        let device_remaps = raw
            .gamepads
            .into_iter()
            .map(parse_device_remap)
            .collect::<Result<_, _>>()?;
        let mut app_rules: HashMap<String, Vec<Rule>> = HashMap::new();
        for (app, rules) in raw.apps.into_iter() {
            let parsed = rules
                .into_iter()
                .map(parse_rule)
                .collect::<Result<Vec<_>, _>>()?;
            app_rules.insert(app, parsed);
        }

        Ok(Self {
            app_rules,
            device_remaps,
        })
    }
}

fn parse_device_remap(raw: RawGamepadRemap) -> Result<DeviceRemap, ProfileError> {
    let vid = parse_vidpid(&raw.vid)
        .ok_or_else(|| ProfileError::Yaml(format!("invalid vid: {:?}", raw.vid)))?;
    let pid = parse_vidpid(&raw.pid)
        .ok_or_else(|| ProfileError::Yaml(format!("invalid pid: {:?}", raw.pid)))?;
    let mut mapping = HashMap::new();
    for (k, v) in raw.mapping.into_iter() {
        let from = parse_button_name(&k)
            .ok_or_else(|| ProfileError::Yaml(format!("invalid button: {k}")))?;
        let to = parse_button_name(&v)
            .ok_or_else(|| ProfileError::Yaml(format!("invalid button: {v}")))?;
        mapping.insert(from, to);
    }
    Ok(DeviceRemap { vid, pid, mapping })
}

fn parse_rule(raw: RawRule) -> Result<Rule, ProfileError> {
    let when = match raw.when.as_deref() {
        Some("pressed") | None => TriggerPhase::Pressed,
        Some("released") => TriggerPhase::Released,
        Some(other) => return Err(ProfileError::Yaml(format!("invalid when: {other}"))),
    };

    // trigger can be string (chord) or mapping for dpad
    let trigger = if raw.trigger.as_str() == Some("dpad") {
        let map = parse_dpad_action(raw.action.clone())?;
        TriggerSpec::Dpad(map)
    } else if raw.trigger.as_str().is_some() {
        let s = raw.trigger.as_str().unwrap();
        TriggerSpec::Chord(
            parse_chord(s).ok_or_else(|| ProfileError::InvalidTrigger(s.to_string()))?,
        )
    } else {
        return Err(ProfileError::InvalidTrigger(format!(
            "unsupported trigger: {:?}",
            raw.trigger
        )));
    };

    let action = match &trigger {
        TriggerSpec::Chord(_) => parse_action(raw.action)?,
        TriggerSpec::Dpad(_) => ActionSpec::None,
    };

    Ok(Rule {
        trigger,
        action,
        vibrate: raw.vibrate,
        when,
    })
}

fn parse_action(val: serde_yaml::Value) -> Result<ActionSpec, ProfileError> {
    if let Some(s) = val.as_str() {
        let kc = s.parse::<KeyCombo>().map_err(ProfileError::Yaml)?;
        return Ok(ActionSpec::Key(kc));
    }
    Err(ProfileError::Yaml("invalid action".into()))
}

fn parse_dpad_action(val: serde_yaml::Value) -> Result<DpadMapping, ProfileError> {
    let map = val
        .as_mapping()
        .ok_or_else(|| ProfileError::Yaml("dpad action must be mapping".into()))?;
    let mut m = DpadMapping {
        up: None,
        down: None,
        left: None,
        right: None,
    };
    for (k, v) in map.iter() {
        let key = k.as_str().unwrap_or("").to_lowercase();
        let act = parse_action(v.clone()).ok().unwrap_or(ActionSpec::None);
        match key.as_str() {
            "up" => m.up = Some(act),
            "down" => m.down = Some(act),
            "left" => m.left = Some(act),
            "right" => m.right = Some(act),
            _ => {}
        }
    }
    Ok(m)
}

fn parse_chord(input: &str) -> Option<HashSet<Button>> {
    let mut set = HashSet::new();
    for part in input.split('+') {
        let name = part.trim().to_lowercase();
        let button = parse_button_name(&name)?;
        set.insert(button);
    }
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

fn parse_button_name(name: &str) -> Option<Button> {
    Some(match name.to_lowercase().as_str() {
        "a" => Button::A,
        "b" => Button::B,
        "x" => Button::X,
        "y" => Button::Y,
        "back" | "select" => Button::Back,
        "guide" | "home" => Button::Guide,
        "start" => Button::Start,
        "ls" | "leftstick" | "left_stick" => Button::LeftStick,
        "rs" | "rightstick" | "right_stick" => Button::RightStick,
        "lb" | "leftshoulder" | "left_shoulder" => Button::LeftShoulder,
        "rb" | "rightshoulder" | "right_shoulder" => Button::RightShoulder,
        "lt" | "lefttrigger" | "left_trigger" => Button::LeftTrigger,
        "rt" | "righttrigger" | "right_trigger" => Button::RightTrigger,
        "dpad_up" | "dpadup" | "up" => Button::DPadUp,
        "dpad_down" | "dpaddown" | "down" => Button::DPadDown,
        "dpad_left" | "dpadleft" | "left" => Button::DPadLeft,
        "dpad_right" | "dpadright" | "right" => Button::DPadRight,
        _ => return None,
    })
}

fn parse_hex_u16(input: &str) -> Option<u16> {
    let s = input.trim().trim_start_matches("0x");
    u16::from_str_radix(s, 16).ok()
}

fn parse_vidpid(v: &serde_yaml::Value) -> Option<u16> {
    if let Some(s) = v.as_str() {
        return parse_hex_u16(s);
    }
    if let Some(n) = v.as_u64() {
        return u16::try_from(n).ok();
    }
    None
}
