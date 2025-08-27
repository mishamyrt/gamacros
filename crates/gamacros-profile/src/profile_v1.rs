use std::collections::HashMap;

use gamacros_controller::Button;
use gamacros_keypress::KeyCombo;
use serde::Deserialize;

use crate::{
    profile::{
        Action, ButtonChord, DeviceRemap, Rule, Profile, Trigger, TriggerPhase,
    },
    ProfileError,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1 {
    version: u8,
    #[serde(default)]
    gamepads: Vec<ProfileV1GamepadRemap>,
    #[serde(default)]
    apps: HashMap<String, Vec<ProfileV1Rule>>, // bundle_id -> rules
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileV1GamepadRemap {
    pub vid: serde_yaml::Value,
    pub pid: serde_yaml::Value,
    #[serde(default)]
    pub mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileV1Rule {
    pub trigger: serde_yaml::Value,
    pub action: serde_yaml::Value,
    #[serde(default)]
    pub vibrate: Option<u16>,
    #[serde(default)]
    pub when: Option<String>,
}

impl ProfileV1 {
    pub fn to_settings(input: &str) -> Result<Profile, ProfileError> {
        let raw: ProfileV1 = serde_yaml::from_str(input)?;
        if raw.version != 1 {
            return Err(ProfileError::UnsupportedVersion(raw.version));
        }
        let device_remaps = raw
            .gamepads
            .into_iter()
            .map(parse_device_remap)
            .collect::<Result<_, _>>()?;

        let mut app_rules: HashMap<Box<str>, Vec<Rule>> = HashMap::new();
        for (app, rules) in raw.apps.into_iter() {
            let parsed = rules
                .into_iter()
                .map(parse_rule)
                .collect::<Result<Vec<_>, _>>()?;
            app_rules.insert(app.into(), parsed);
        }

        Ok(Profile {
            app_rules,
            device_remaps,
        })
    }
}

fn parse_device_remap(
    raw: ProfileV1GamepadRemap,
) -> Result<DeviceRemap, ProfileError> {
    let vid = parse_vidpid(&raw.vid).ok_or_else(|| {
        ProfileError::InvalidId("vid".to_string(), format!("{:?}", raw.vid))
    })?;
    let pid = parse_vidpid(&raw.pid).ok_or_else(|| {
        ProfileError::InvalidId("pid".to_string(), format!("{:?}", raw.pid))
    })?;
    let mut mapping = HashMap::new();
    for (k, v) in raw.mapping.into_iter() {
        let from = parse_button_name(&k)?;
        let to = parse_button_name(&v)?;
        mapping.insert(from, to);
    }
    Ok(DeviceRemap { vid, pid, mapping })
}

fn parse_rule(raw: ProfileV1Rule) -> Result<Rule, ProfileError> {
    let when = match raw.when.as_deref() {
        Some("pressed") | None => TriggerPhase::Pressed,
        Some("released") => TriggerPhase::Released,
        Some(other) => {
            return Err(ProfileError::InvalidTrigger(format!(
                "invalid when: {other}"
            )))
        }
    };

    // trigger can be string (chord) or mapping for dpad
    let trigger = if raw.trigger.as_str().is_some() {
        let s = raw.trigger.as_str().unwrap();
        Trigger::Chord(parse_chord(s)?)
    } else {
        return Err(ProfileError::InvalidTrigger(format!(
            "unsupported trigger: {:?}",
            raw.trigger
        )));
    };

    let action = match &trigger {
        Trigger::Chord(_) => parse_action(raw.action)?,
    };

    Ok(Rule {
        trigger,
        action,
        vibrate: raw.vibrate,
        when,
    })
}

fn parse_action(val: serde_yaml::Value) -> Result<Action, ProfileError> {
    if let Some(s) = val.as_str() {
        let kc = s
            .parse::<KeyCombo>()
            .map_err(ProfileError::KeyParseError)?;
        return Ok(Action::Key(kc));
    }
    Err(ProfileError::InvalidTrigger("invalid action".into()))
}

fn parse_chord(input: &str) -> Result<ButtonChord, ProfileError> {
    let mut set = ButtonChord::empty();
    for part in input.split('+') {
        let name = part.trim().to_lowercase();
        let button = parse_button_name(&name)?;
        set.insert(button);
    }
    if set.is_empty() {
        Err(ProfileError::InvalidTrigger(input.to_string()))
    } else {
        Ok(set)
    }
}

fn parse_button_name(name: &str) -> Result<Button, ProfileError> {
    Ok(match name.to_lowercase().as_str() {
        "a" => Button::A,
        "b" => Button::B,
        "x" => Button::X,
        "y" => Button::Y,
        "back" | "select" => Button::Back,
        "guide" | "home" => Button::Guide,
        "start" => Button::Start,
        "ls" | "leftstick" | "left_stick" => Button::LeftStick,
        "rs" | "rightstick" | "right_stick" => Button::RightStick,
        "lb" | "left_bump" | "leftshoulder"  | "left_shoulder" | "l1" => Button::LeftShoulder,
        "rb" | "right_bump" | "rightshoulder" | "right_shoulder" | "r1" => Button::RightShoulder,
        "lt" | "lefttrigger" | "left_trigger" | "l2" => Button::LeftTrigger,
        "rt" | "righttrigger" | "right_trigger" | "r2" => Button::RightTrigger,
        "dpad_up" | "dpadup" | "up" => Button::DPadUp,
        "dpad_down" | "dpaddown" | "down" => Button::DPadDown,
        "dpad_left" | "dpadleft" | "left" => Button::DPadLeft,
        "dpad_right" | "dpadright" | "right" => Button::DPadRight,
        _ => return Err(ProfileError::InvalidButton(name.to_string())),
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
