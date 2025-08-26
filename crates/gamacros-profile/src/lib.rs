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
pub enum TriggerPhase { Pressed, Released }

#[derive(Debug, Clone)]
pub struct RuntimeAction {
    pub key_combo: Option<KeyCombo>,
    pub vibrate_ms: Option<u16>,
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
    app_rules: HashMap<String, Vec<Rule>>, // bundle_id -> rules
    device_remaps: Vec<DeviceRemap>,
}

#[derive(Debug, Clone)]
struct DeviceRemap {
    vid: u16,
    pid: u16,
    mapping: HashMap<Button, Button>,
}

#[derive(Debug, Clone)]
struct Rule {
    trigger: TriggerSpec,
    action: ActionSpec,
    vibrate: Option<u16>,
    when: TriggerPhase,
}

#[derive(Debug, Clone)]
enum TriggerSpec {
    Chord(HashSet<Button>),
    Dpad(DpadMapping),
}

#[derive(Debug, Clone)]
struct DpadMapping {
    up: Option<ActionSpec>,
    down: Option<ActionSpec>,
    left: Option<ActionSpec>,
    right: Option<ActionSpec>,
}

#[derive(Debug, Clone)]
enum ActionSpec {
    Key(KeyCombo),
    None,
}

impl Profile {
    pub fn empty() -> Self {
        Self { app_rules: HashMap::new(), device_remaps: Vec::new() }
    }

    pub fn from_yaml_str(input: &str) -> Result<Self, ProfileError> {
        let raw: RawProfile = serde_yaml::from_str(input).map_err(|e| ProfileError::Yaml(e.to_string()))?;
        if raw.version != 1 { return Err(ProfileError::Yaml("unsupported version".into())); }

        let device_remaps = raw.gamepads.into_iter().map(parse_device_remap).collect::<Result<_,_>>()?;
        let mut app_rules: HashMap<String, Vec<Rule>> = HashMap::new();
        for (app, rules) in raw.apps.into_iter() {
            let parsed = rules.into_iter().map(parse_rule).collect::<Result<Vec<_>,_>>()?;
            app_rules.insert(app, parsed);
        }

        Ok(Self { app_rules, device_remaps })
    }

    pub fn build_runtime(&self, vid: u16, pid: u16, app: &str) -> ProfileRuntime {
        let remap = self.device_remaps.iter().find(|r| r.vid == vid && r.pid == pid);
        let rules = self.app_rules.get(app).cloned().unwrap_or_default();
        ProfileRuntime {
            current_app: app.to_string(),
            device_mapping: remap.map(|r| r.mapping.clone()).unwrap_or_default(),
            pressed: HashSet::new(),
            rules,
            all_rules: self.app_rules.clone(),
        }
    }
}

pub struct ProfileRuntime {
    current_app: String,
    device_mapping: HashMap<Button, Button>,
    pressed: HashSet<Button>,
    rules: Vec<Rule>,
    all_rules: HashMap<String, Vec<Rule>>, // app -> rules
}

impl ProfileRuntime {
    pub fn set_app(&mut self, app: &str) {
        self.current_app = app.to_string();
        self.rules = self.all_rules.get(app).cloned().unwrap_or_default();
        // Reset pressed state on app switch to avoid stale chords
        self.pressed.clear();
    }

    pub fn on_button(&mut self, button: Button, phase: TriggerPhase) -> Vec<RuntimeAction> {
        let mapped = self.device_mapping.get(&button).cloned().unwrap_or(button);
        // Evaluate against an appropriate view of the pressed set
        let prev = self.pressed.clone();
        let eval_set: HashSet<Button> = match phase {
            TriggerPhase::Pressed => {
                let mut after = self.pressed.clone();
                after.insert(mapped);
                after
            }
            TriggerPhase::Released => prev.clone(),
        };
        // Now mutate real state
        match phase {
            TriggerPhase::Pressed => { self.pressed.insert(mapped); }
            TriggerPhase::Released => { self.pressed.remove(&mapped); }
        }

        let mut actions = Vec::new();
        for rule in self.rules.iter() {
            if rule.when != phase { continue; }
            match &rule.trigger {
                TriggerSpec::Chord(ch) => {
                    if ch.is_subset(&eval_set) {
                        if let Some(act) = rule.action.to_runtime_action(rule.vibrate) { actions.push(act); }
                    }
                }
                TriggerSpec::Dpad(map) => {
                    let maybe = match mapped {
                        Button::DPadUp => map.up.as_ref(),
                        Button::DPadDown => map.down.as_ref(),
                        Button::DPadLeft => map.left.as_ref(),
                        Button::DPadRight => map.right.as_ref(),
                        _ => None,
                    };
                    if let Some(action) = maybe {
                        if let Some(act) = action.to_runtime_action(rule.vibrate) { actions.push(act); }
                    }
                }
            }
        }
        actions
    }
}

fn parse_device_remap(raw: RawGamepadRemap) -> Result<DeviceRemap, ProfileError> {
    let vid = parse_vidpid(&raw.vid).ok_or_else(|| ProfileError::Yaml(format!("invalid vid: {:?}", raw.vid)))?;
    let pid = parse_vidpid(&raw.pid).ok_or_else(|| ProfileError::Yaml(format!("invalid pid: {:?}", raw.pid)))?;
    let mut mapping = HashMap::new();
    for (k, v) in raw.mapping.into_iter() {
        let from = parse_button_name(&k).ok_or_else(|| ProfileError::Yaml(format!("invalid button: {k}")))?;
        let to = parse_button_name(&v).ok_or_else(|| ProfileError::Yaml(format!("invalid button: {v}")))?;
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
        TriggerSpec::Chord(parse_chord(s).ok_or_else(|| ProfileError::InvalidTrigger(s.to_string()))?)
    } else {
        return Err(ProfileError::InvalidTrigger(format!("unsupported trigger: {:?}", raw.trigger)));
    };

    let action = match &trigger {
        TriggerSpec::Chord(_) => parse_action(raw.action)?,
        TriggerSpec::Dpad(_) => ActionSpec::None,
    };

    Ok(Rule { trigger, action, vibrate: raw.vibrate, when })
}

fn parse_action(val: serde_yaml::Value) -> Result<ActionSpec, ProfileError> {
    if let Some(s) = val.as_str() {
        let kc = s.parse::<KeyCombo>().map_err(ProfileError::Yaml)?;
        return Ok(ActionSpec::Key(kc));
    }
    Err(ProfileError::Yaml("invalid action".into()))
}

fn parse_dpad_action(val: serde_yaml::Value) -> Result<DpadMapping, ProfileError> {
    let map = val.as_mapping().ok_or_else(|| ProfileError::Yaml("dpad action must be mapping".into()))?;
    let mut m = DpadMapping { up: None, down: None, left: None, right: None };
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
    if set.is_empty() { None } else { Some(set) }
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

impl ActionSpec {
    fn to_runtime_action(&self, vibrate: Option<u16>) -> Option<RuntimeAction> {
        match self {
            ActionSpec::Key(k) => Some(RuntimeAction { key_combo: Some(k.clone()), vibrate_ms: vibrate }),
            ActionSpec::None => None,
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn load_profile(yaml: &str) -> Profile {
        Profile::from_yaml_str(yaml).expect("parse profile")
    }

    #[test]
    fn parse_basic_profile_and_chord() {
        let yaml = r#"
version: 1
apps:
  com.example.app:
    - trigger: lt+rt
      action: cmd+shift+l
"#;
        let profile = load_profile(yaml);
        let mut rt = profile.build_runtime(0, 0, "com.example.app");

        // Press LT then RT -> should match chord exactly when both held
        let mut actions = rt.on_button(Button::LeftTrigger, TriggerPhase::Pressed);
        assert!(actions.is_empty());
        actions = rt.on_button(Button::RightTrigger, TriggerPhase::Pressed);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].key_combo.is_some());

        // Release does not fire (default when: pressed)
        actions = rt.on_button(Button::LeftTrigger, TriggerPhase::Released);
        assert!(actions.is_empty());
        actions = rt.on_button(Button::RightTrigger, TriggerPhase::Released);
        assert!(actions.is_empty());
    }

    #[test]
    fn dpad_mapping_actions() {
        let yaml = r#"
version: 1
apps:
  com.example.app:
    - trigger: dpad
      action:
        up: arrow_up
        down: arrow_down
        left: arrow_left
        right: arrow_right
"#;
        let profile = load_profile(yaml);
        let mut rt = profile.build_runtime(0, 0, "com.example.app");

        for (button, label) in [
            (Button::DPadUp, "up"),
            (Button::DPadDown, "down"),
            (Button::DPadLeft, "left"),
            (Button::DPadRight, "right"),
        ] {
            let actions = rt.on_button(button, TriggerPhase::Pressed);
            assert_eq!(actions.len(), 1, "expected action for dpad {label}");
            assert!(actions[0].key_combo.is_some());
        }
    }

    #[test]
    fn vid_pid_remap_swaps_buttons() {
        let yaml = r#"
version: 1
gamepads:
  - vid: 0x57e
    pid: 0x2009
    mapping: { a: b, b: a, x: y, y: x }
apps:
  com.example.app:
    - trigger: a
      action: enter
"#;
        let profile = load_profile(yaml);
        // With Nintendo-style swap, pressing physical B maps to canonical A
        let mut rt = profile.build_runtime(0x057e, 0x2009, "com.example.app");
        let actions = rt.on_button(Button::B, TriggerPhase::Pressed);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].key_combo.is_some());
    }

    #[test]
    fn when_released_only() {
        let yaml = r#"
version: 1
apps:
  com.example.app:
    - trigger: a
      when: released
      action: space
"#;
        let profile = load_profile(yaml);
        let mut rt = profile.build_runtime(0, 0, "com.example.app");

        let actions = rt.on_button(Button::A, TriggerPhase::Pressed);
        assert!(actions.is_empty());
        let actions = rt.on_button(Button::A, TriggerPhase::Released);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn app_switching_updates_rules() {
        let yaml = r#"
version: 1
apps:
  com.example.a:
    - trigger: a
      action: enter
  com.example.b:
    - trigger: b
      action: tab
"#;
        let profile = load_profile(yaml);
        let mut rt = profile.build_runtime(0, 0, "com.example.a");

        // In app A, A triggers
        let actions = rt.on_button(Button::A, TriggerPhase::Pressed);
        assert_eq!(actions.len(), 1);
        // In app B, B triggers and A no longer does
        rt.set_app("com.example.b");
        let actions = rt.on_button(Button::A, TriggerPhase::Pressed);
        assert!(actions.is_empty());
        let actions = rt.on_button(Button::B, TriggerPhase::Pressed);
        assert_eq!(actions.len(), 1);
    }
}
