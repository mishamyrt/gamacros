use std::{collections::HashMap, sync::RwLock};
use dashmap::DashMap;
use gamacros_bit_mask::AtomicBitmask;
use thiserror::Error;

use gamacros_controller::{Button, ControllerId};
use gamacros_profile::{Profile, Rule, Trigger, TriggerPhase};

#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("failed to lock active app")]
    FailedToLockActiveApp,
}
type Result<T> = std::result::Result<T, ManagerError>;

#[derive(Debug, Clone, Default)]
pub struct ControllerMapping(HashMap<Button, Button>);

impl ControllerMapping {
    pub fn new(mapping: HashMap<Button, Button>) -> Self {
        Self(mapping)
    }

    pub fn get(&self, button: Button) -> Button {
        self.0.get(&button).cloned().unwrap_or(button)
    }
}

#[derive(Debug)]
struct ControllerState {
    mapping: ControllerMapping,
    pressed: AtomicBitmask<Button>,
}

pub struct Gamacros {
    profile: Profile,
    active_app: RwLock<Box<str>>,
    devices: DashMap<ControllerId, ControllerState>,
}

impl Gamacros {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            active_app: RwLock::new("".into()),
            devices: DashMap::new(),
        }
    }

    pub fn add_device(&self, id: ControllerId, vid: u16, pid: u16) -> Result<()> {
        println!("gamacros: add device - {id}");
        let mapping = self
            .profile
            .device_remaps
            .iter()
            .find(|d| d.vid == vid && d.pid == pid)
            .map(|d| ControllerMapping::new(d.mapping.clone()))
            .unwrap_or_default();
        let state = ControllerState {
            mapping,
            pressed: AtomicBitmask::empty(),
        };
        self.devices.insert(id, state);
        Ok(())
    }

    pub fn remove_device(&self, id: ControllerId) -> Result<()> {
        println!("gamacros: remove device - {id}");
        self.devices.remove(&id);
        Ok(())
    }

    pub fn set_active_app(&self, app: &str) -> Result<()> {
        println!("gamacros: app change - {app}");
        match self.active_app.write() {
            Ok(mut active_app) => {
                *active_app = app.into();
                Ok(())
            }
            Err(_) => Err(ManagerError::FailedToLockActiveApp),
        }
    }

    pub fn get_active_app(&self) -> Box<str> {
        self.active_app
            .read()
            .expect("failed to lock active app")
            .clone()
    }

    pub fn handle_button(
        &self,
        id: ControllerId,
        button: Button,
        phase: TriggerPhase,
    ) -> Vec<Rule> {
        let active_app = self.get_active_app();
        let Some(app_rules) = self.profile.app_rules.get(&active_app).cloned()
        else {
            return vec![];
        };
        let state_ref = self
            .devices
            .get(&id)
            .expect("device must be added before use");
        let state = state_ref.value();
        let button = state.mapping.get(button);

        // snapshot before change
        let prev_pressed = state.pressed.load();

        if phase == TriggerPhase::Pressed {
            state.pressed.insert(button);
        } else {
            state.pressed.remove(button);
        }

        // snapshot after change
        let now_pressed = state.pressed.load();

        let mut candidates: Vec<(Rule, u32)> = vec![];

        for rule in app_rules.iter() {
            match &rule.trigger {
                Trigger::Chord(target) => {
                    let was = prev_pressed.is_superset(target);
                    let is_now = now_pressed.is_superset(target);

                    let fire = match rule.when {
                        // For Pressed rules, fire on both edges (activation and deactivation)
                        // so that main.rs can press on activation (phase=Pressed)
                        // and release on deactivation (phase=Released).
                        TriggerPhase::Pressed => was != is_now,
                        TriggerPhase::Released => was && !is_now,
                    };

                    if fire {
                        let bits: u32 = target.count();
                        candidates.push((rule.clone(), bits));
                    }
                }
            }
        }

        if candidates.is_empty() {
            return vec![];
        }

        let max_bits = candidates.iter().map(|(_, b)| *b).max().unwrap_or(0);
        let actions: Vec<Rule> = candidates
            .into_iter()
            .filter(|(_, b)| *b == max_bits)
            .map(|(r, _)| r)
            .collect();

        actions
    }
}

