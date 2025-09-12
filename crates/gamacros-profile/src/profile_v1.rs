use std::collections::HashMap;
use std::sync::Arc;
use ahash::AHashMap;
use serde::Deserialize;

use gamacros_control::KeyCombo;

use crate::selector::Selector;

use crate::{BundleId, ControllerSettingsMap, DEFAULT_SHELL};
use crate::{
    profile::{
        AppRules, ArrowsParams, Axis, ButtonChord, ButtonRule, ButtonRules,
        ControllerSettings, MouseParams, Profile, ScrollParams, StepperParams,
        StickMode, StickRules, StickSide,
    },
    profile_common::parse_button_name,
    ButtonAction, ProfileError,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileV1 {
    version: u8,
    #[serde(default)]
    controllers: Vec<ProfileV1ControllerParams>,
    #[serde(default)]
    blacklist: Vec<String>,
    #[serde(default)]
    groups: AHashMap<String, Vec<Box<str>>>,
    #[serde(default)]
    rules: AHashMap<Box<str>, ProfileV1App>, // bundle_id -> app mapping
    #[serde(default)]
    shell: Option<Box<str>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ProfileV1App {
    #[serde(default)]
    pub buttons: AHashMap<String, ProfileV1ButtonRule>, // chord -> button rule
    #[serde(default)]
    pub sticks: AHashMap<String, ProfileV1Stick>, // side -> stick rules
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileV1ButtonRule {
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
struct ProfileV1ControllerParams {
    pub vid: u16,
    pub pid: u16,
    #[serde(default)]
    pub remap: HashMap<String, String>, // button -> button
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileV1Stick {
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

const COMMON_BUNDLE_ID: &str = "common";

impl ProfileV1 {
    pub fn to_settings(input: &str) -> Result<Profile, ProfileError> {
        let mut raw: ProfileV1 = serde_yaml::from_str(input)?;
        if raw.version != 1 {
            return Err(ProfileError::UnsupportedVersion(raw.version));
        }

        let mut rules: AHashMap<BundleId, AppRules> = AHashMap::new();

        let common_rules = raw
            .rules
            .remove(COMMON_BUNDLE_ID)
            .map(|r| parse_app_rules(r, COMMON_BUNDLE_ID))
            .transpose()?;

        if let Some(common_rules) = common_rules.clone() {
            rules.insert(COMMON_BUNDLE_ID.into(), common_rules);
        }

        for (selector, app_actions) in raw.rules.into_iter() {
            let parsed_selector = Selector::parse(&selector)?;
            let bundle_ids = parsed_selector.materialize(&raw.groups)?;
            let app_rules = parse_app_rules(app_actions, &selector)?;

            for bundle_id in bundle_ids {
                // Using common rules as default. If there are no common rules, use empty rules.
                // If there are common rules, merge them with the app rules.
                let current_rules = {
                    if let Some(current_rules) = rules.get_mut(&bundle_id) {
                        current_rules.buttons.extend(app_rules.buttons.clone());
                        current_rules.sticks.extend(app_rules.sticks.clone());

                        current_rules.clone()
                    } else {
                        let mut default_rules =
                            common_rules.clone().unwrap_or_default();
                        default_rules.buttons.extend(app_rules.buttons.clone());
                        default_rules.sticks.extend(app_rules.sticks.clone());

                        rules.insert(bundle_id.clone(), default_rules.clone());
                        default_rules
                    }
                };

                rules.insert(bundle_id, current_rules);
            }
        }

        let controllers = parse_controller_settings(raw.controllers)?;
        let blacklist = raw.blacklist.into_iter().collect();

        Ok(Profile {
            blacklist,
            controllers,
            rules,
            shell: raw.shell.unwrap_or(DEFAULT_SHELL.into()),
        })
    }
}

fn parse_app_rules(
    raw: ProfileV1App,
    bundle_id: &str,
) -> Result<AppRules, ProfileError> {
    let mut button_rules: ButtonRules = AHashMap::new();
    let mut stick_rules: StickRules = AHashMap::new();

    for (chord_str, rule) in raw.buttons.into_iter() {
        let chord = parse_chord(&chord_str)?;
        let rule = parse_button_rule(rule, bundle_id)?;
        button_rules.insert(chord, rule);
    }

    for (side, stick_raw) in raw.sticks.into_iter() {
        let side = StickSide::parse(&side)?;
        let mode = parse_stick_mode(stick_raw)?;
        stick_rules.insert(side, mode);
    }

    Ok(AppRules {
        buttons: button_rules,
        sticks: stick_rules,
    })
}

fn parse_controller_settings(
    raw: Vec<ProfileV1ControllerParams>,
) -> Result<ControllerSettingsMap, ProfileError> {
    let mut settings: ControllerSettingsMap = AHashMap::new();
    for raw_settings in raw {
        let device_id = (raw_settings.vid, raw_settings.pid);
        let device_settings = parse_device_remap(raw_settings)?;
        settings.insert(device_id, device_settings);
    }
    Ok(settings)
}

fn parse_device_remap(
    raw: ProfileV1ControllerParams,
) -> Result<ControllerSettings, ProfileError> {
    let mut remap = AHashMap::new();
    for (k, v) in raw.remap.into_iter() {
        let from = parse_button_name(&k)?;
        let to = parse_button_name(&v)?;
        remap.insert(from, to);
    }
    Ok(ControllerSettings { mapping: remap })
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

fn parse_keystroke(input: &str) -> Result<KeyCombo, ProfileError> {
    input
        .parse::<KeyCombo>()
        .map_err(ProfileError::KeyParseError)
}

fn parse_macros(input: &[String]) -> Result<Vec<KeyCombo>, ProfileError> {
    input
        .iter()
        .map(|m| m.as_str())
        .map(parse_keystroke)
        .collect::<Result<Vec<_>, _>>()
}

fn parse_button_rule(
    raw: ProfileV1ButtonRule,
    target_name: &str,
) -> Result<ButtonRule, ProfileError> {
    let action = match (raw.keystroke, raw.macros, raw.shell) {
        (Some(keystroke), None, None) => {
            let keystroke = parse_keystroke(&keystroke)?;
            ButtonAction::Keystroke(Arc::new(keystroke))
        }
        (None, Some(macros), None) => {
            let macros = parse_macros(&macros)?;
            ButtonAction::Macros(Arc::new(macros))
        }
        (None, None, Some(shell)) => ButtonAction::Shell(shell),
        _ => return Err(ProfileError::InvalidActions(target_name.to_string())),
    };

    Ok(ButtonRule {
        vibrate: raw.vibrate,
        action,
    })
}

fn parse_stick_mode(raw: ProfileV1Stick) -> Result<StickMode, ProfileError> {
    let deadzone = raw.deadzone.unwrap_or(0.15);
    let mode = match raw.mode.to_lowercase().as_str() {
        "arrows" => {
            let params = ArrowsParams {
                deadzone,
                repeat_delay_ms: raw.repeat_delay_ms.unwrap_or(300),
                repeat_interval_ms: raw.repeat_interval_ms.unwrap_or(40),
                invert_x: raw.invert_x.unwrap_or(false),
                invert_y: raw.invert_y.unwrap_or(false),
            };
            StickMode::Arrows(params)
        }
        "mouse_move" => {
            let params = MouseParams {
                deadzone,
                max_speed_px_s: raw.max_speed_px_s.unwrap_or(1600.0),
                gamma: raw.gamma.unwrap_or(1.5),
                invert_x: raw.invert_x.unwrap_or(false),
                invert_y: raw.invert_y.unwrap_or(false),
            };
            StickMode::MouseMove(params)
        }
        "scroll" => {
            let params = ScrollParams {
                deadzone,
                speed_lines_s: raw.speed_lines_s.unwrap_or(100.0),
                horizontal: raw.horizontal.unwrap_or(false),
                invert_x: raw.invert_x.unwrap_or(false),
                invert_y: raw.invert_y.unwrap_or(false),
            };
            StickMode::Scroll(params)
        }
        "volume" => {
            let axis =
                match raw.axis.as_deref().unwrap_or("y").to_lowercase().as_str() {
                    "x" => Axis::X,
                    "y" => Axis::Y,
                    other => {
                        return Err(ProfileError::InvalidTrigger(format!(
                            "invalid axis: {other}"
                        )))
                    }
                };
            let params = StepperParams {
                axis,
                deadzone,
                invert: raw.invert.unwrap_or(false),
                min_interval_ms: raw.min_interval_ms.unwrap_or(250),
                max_interval_ms: raw.max_interval_ms.unwrap_or(40),
            };
            StickMode::Volume(params)
        }
        "brightness" => {
            let axis =
                match raw.axis.as_deref().unwrap_or("y").to_lowercase().as_str() {
                    "x" => Axis::X,
                    "y" => Axis::Y,
                    other => {
                        return Err(ProfileError::InvalidTrigger(format!(
                            "invalid axis: {other}"
                        )))
                    }
                };
            let params = StepperParams {
                axis,
                deadzone,
                invert: raw.invert.unwrap_or(false),
                min_interval_ms: raw.min_interval_ms.unwrap_or(250),
                max_interval_ms: raw.max_interval_ms.unwrap_or(40),
            };
            StickMode::Brightness(params)
        }
        other => {
            return Err(ProfileError::InvalidTrigger(format!(
                "invalid stick mode: {other}"
            )))
        }
    };

    Ok(mode)
}
