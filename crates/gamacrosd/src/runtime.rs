use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use gamacros_controller::Button;
use gamacros_keypress::KeyCombo;
use gamacros_profile::{ActionSpec, Profile, Rule, TriggerPhase, TriggerSpec};

pub fn build_runtime(profile: &Profile, vid: u16, pid: u16, app: &str) -> ProfileRuntime {
    let remap = profile
        .device_remaps
        .iter()
        .find(|r| r.vid == vid && r.pid == pid);
    let rules = profile.app_rules.get(app).cloned().unwrap_or_default();
    ProfileRuntime {
        pressed: Mutex::new(HashSet::new()),
        current_app: app.to_string().into(),
        device_mapping: remap.map(|r| r.mapping.clone()).unwrap_or_default(),
        rules,
        all_rules: profile.app_rules.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeAction {
    pub key_combo: Option<KeyCombo>,
    pub vibrate_ms: Option<u16>,
}

fn to_runtime_action(spec: ActionSpec, vibrate: Option<u16>) -> Option<RuntimeAction> {
    match spec {
        ActionSpec::Key(k) => Some(RuntimeAction {
            key_combo: Some(k.clone()),
            vibrate_ms: vibrate,
        }),
        ActionSpec::None => None,
    }
}

pub struct ProfileRuntime {
    current_app: Mutex<String>,
    pressed: Mutex<HashSet<Button>>,
    device_mapping: HashMap<Button, Button>,
    rules: Vec<Rule>,
    all_rules: HashMap<String, Vec<Rule>>, // app -> rules
}

impl ProfileRuntime {
    pub fn set_app(&mut self, app: &str) {
        let Ok(mut current_app) = self.current_app.lock() else {
            eprintln!("Failed to lock current_app");
            return;
        };
        *current_app = app.to_string();
        let mut pressed = self.pressed.lock().unwrap();
        pressed.clear();
        self.rules = self.all_rules.get(app).cloned().unwrap_or_default();
    }

    pub fn on_button(&mut self, button: Button, phase: TriggerPhase) -> Vec<RuntimeAction> {
        let mapped = self.device_mapping.get(&button).cloned().unwrap_or(button);
        let mut actions = Vec::new();
        let mut pressed = self.pressed.lock().unwrap();
        if phase == TriggerPhase::Pressed {
            pressed.insert(button);
        } else {
            pressed.remove(&button);
        }
        for rule in self.rules.iter() {
            if rule.when != phase {
                continue;
            }
            match &rule.trigger {
                TriggerSpec::Chord(target) => {
                    if target.is_subset(&pressed) {
                        if let Some(act) = to_runtime_action(rule.action.clone(), rule.vibrate) {
                            actions.push(act);
                        }
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
                        if let Some(act) = to_runtime_action(action.clone(), rule.vibrate) {
                            actions.push(act);
                        }
                    }
                }
            }
        }
        actions
    }
}
