use std::sync::Arc;

use ahash::AHashMap;
use gamacros_control::KeyCombo;
use gamacros_gamepad::Button;

use crate::v1::profile::{ProfileV1ButtonRule, ProfileV1Stick};
use crate::profile::{
    AppRules, ArrowsParams, Axis, ButtonAction, ButtonRule, ButtonRules,
    ControllerSettings, ControllerSettingsMap, Macros, MouseParams, Profile,
    RuleMap, ScrollParams, StepperParams, StickMode, StickRules, StickSide,
};
use crate::ButtonChord;

use super::Error;
use super::profile::{ProfileV1, ProfileV1App, ProfileV1ControllerSettings};
use super::strings::COMMON_BUNDLE_ID;
use super::selector::Selector;
use super::combo::parse_terms_with_delim;

impl ProfileV1 {
    pub fn parse(&self) -> Result<Profile, Error> {
        if self.version != 1 {
            // This code point should never be reached.
            panic!("unsupported version: {}", self.version);
        }

        let mut rules: RuleMap = AHashMap::new();

        let common_rules = self
            .rules
            .get(COMMON_BUNDLE_ID)
            .map(|r| parse_app_rules(r.clone(), COMMON_BUNDLE_ID))
            .transpose()?;

        if let Some(common_rules) = common_rules.clone() {
            rules.insert(COMMON_BUNDLE_ID.into(), common_rules);
        }

        for (selector, app_actions) in self.rules.clone().into_iter() {
            let parsed_selector = Selector::parse(&selector)?;
            let bundle_ids = parsed_selector.materialize(&self.groups)?;
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

        let controllers = parse_controller_settings(&self.controllers)?;
        let blacklist = self.blacklist.clone().into_iter().collect();

        Ok(Profile {
            blacklist,
            controllers,
            rules,
            shell: self.shell.clone(),
        })
    }
}

fn parse_controller_settings(
    raw: &Vec<ProfileV1ControllerSettings>,
) -> Result<ControllerSettingsMap, Error> {
    let mut settings: ControllerSettingsMap = AHashMap::new();
    for raw_settings in raw {
        let device_id = (raw_settings.vid, raw_settings.pid);
        let device_settings = parse_device_remap(raw_settings)?;
        settings.insert(device_id, device_settings);
    }
    Ok(settings)
}

/// Parse a v1 device remap.
fn parse_device_remap(
    raw: &ProfileV1ControllerSettings,
) -> Result<ControllerSettings, Error> {
    let mut remap = AHashMap::new();
    for (k, v) in raw.remap.iter() {
        let from = parse_button_name(k)?;
        let to = parse_button_name(v)?;
        remap.insert(from, to);
    }
    Ok(ControllerSettings { mapping: remap })
}

/// Parse a button name into a `Button` enum.
fn parse_button_name(name: &str) -> Result<Button, Error> {
    Ok(match name {
        "a" => Button::A,
        "b" => Button::B,
        "x" => Button::X,
        "y" => Button::Y,

        "back" | "select" => Button::Back,
        "guide" | "home" => Button::Guide,
        "start" => Button::Start,

        "ls" | "left_stick" => Button::LeftStick,
        "rs" | "right_stick" => Button::RightStick,

        "lb" | "left_bumper" | "left_shoulder" | "l1" => Button::LeftShoulder,
        "rb" | "right_bumper" | "right_shoulder" | "r1" => Button::RightShoulder,
        "lt" | "left_trigger" | "l2" => Button::LeftTrigger,
        "rt" | "right_trigger" | "r2" => Button::RightTrigger,

        "dpad_up" => Button::DPadUp,
        "dpad_down" => Button::DPadDown,
        "dpad_left" => Button::DPadLeft,
        "dpad_right" => Button::DPadRight,

        _ => return Err(Error::InvalidButton(name.to_string())),
    })
}

/// Parse a v1 app rules.
fn parse_app_rules(raw: ProfileV1App, bundle_id: &str) -> Result<AppRules, Error> {
    let mut button_rules: ButtonRules = AHashMap::new();
    let mut stick_rules: StickRules = AHashMap::new();

    for (chord_str, rule) in raw.buttons.into_iter() {
        let chord = parse_chord(&chord_str)?;
        let rule = parse_button_rule(rule, bundle_id)?;
        button_rules.insert(chord, rule);
    }

    for (side, stick_raw) in raw.sticks.into_iter() {
        let side = parse_stick_side(&side)?;
        let mode = parse_stick_mode(stick_raw)?;
        stick_rules.insert(side, mode);
    }

    Ok(AppRules {
        buttons: button_rules,
        sticks: stick_rules,
    })
}

fn parse_stick_side(raw: &str) -> Result<StickSide, Error> {
    Ok(match raw {
        "left" => StickSide::Left,
        "right" => StickSide::Right,
        other => return Err(Error::InvalidStickSide(other.to_string())),
    })
}

fn parse_chord(input: &str) -> Result<ButtonChord, Error> {
    let mut set = ButtonChord::empty();
    for term in parse_terms_with_delim(input, '+')
        .map_err(|e| Error::InvalidTrigger(format!("{input}: {e:?}")))?
    {
        let button = parse_button_name(term.trim())?;
        set.insert(button);
    }
    if set.is_empty() {
        Err(Error::InvalidTrigger(input.to_string()))
    } else {
        Ok(set)
    }
}

fn parse_button_rule(
    raw: ProfileV1ButtonRule,
    target_name: &str,
) -> Result<ButtonRule, Error> {
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
        _ => return Err(Error::InvalidActions(target_name.to_string())),
    };

    Ok(ButtonRule {
        vibrate: raw.vibrate,
        action,
    })
}

fn parse_keystroke(input: &str) -> Result<KeyCombo, Error> {
    input.parse::<KeyCombo>().map_err(Error::KeyParse)
}

fn parse_macros(input: &[String]) -> Result<Macros, Error> {
    input
        .iter()
        .map(|m| m.as_str())
        .map(parse_keystroke)
        .collect::<Result<Macros, _>>()
}

fn parse_stick_mode(raw: ProfileV1Stick) -> Result<StickMode, Error> {
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
                        return Err(Error::InvalidTrigger(format!(
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
                        return Err(Error::InvalidTrigger(format!(
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
            return Err(Error::InvalidTrigger(format!(
                "invalid stick mode: {other}"
            )))
        }
    };

    Ok(mode)
}
