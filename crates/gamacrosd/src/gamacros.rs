use std::{collections::HashMap, sync::RwLock};

use dashmap::DashMap;
use thiserror::Error;
use colored::Colorize;

use crate::{print_debug, print_info};

use gamacros_bit_mask::AtomicBitmask;
use gamacros_controller::{Button, ControllerId, ControllerInfo};
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
    rumble: bool,
}

pub struct Gamacros {
    profile: Profile,
    active_app: RwLock<Box<str>>,
    controllers: DashMap<ControllerId, ControllerState>,
}

impl Gamacros {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            active_app: RwLock::new("".into()),
            controllers: DashMap::new(),
        }
    }

    pub fn is_known(&self, id: ControllerId) -> bool {
        self.controllers.contains_key(&id)
    }

    pub fn add_controller(&self, info: ControllerInfo) -> Result<()> {
        print_info!(
            "add controller - {0} id={1} vid=0x{2:x} pid=0x{3:x}",
            info.name,
            info.id,
            info.vendor_id,
            info.product_id
        );
        let mapping = self
            .profile
            .device_remaps
            .iter()
            .find(|d| d.vid == info.vendor_id && d.pid == info.product_id)
            .map(|d| ControllerMapping::new(d.mapping.clone()))
            .unwrap_or_default();
        let state = ControllerState {
            mapping,
            pressed: AtomicBitmask::empty(),
            rumble: info.supports_rumble,
        };
        if self.is_known(info.id) {
            print_debug!("controller already known - id={0}", info.id);
        }
        self.controllers.insert(info.id, state);
        Ok(())
    }

    pub fn remove_controller(&self, id: ControllerId) -> Result<()> {
        print_info!("remove device - {id:x}");
        self.controllers.remove(&id);
        Ok(())
    }

    pub fn supports_rumble(&self, id: ControllerId) -> bool {
        self.controllers.get(&id).map(|s| s.rumble).unwrap_or(false)
    }

    pub fn set_active_app(&self, app: &str) -> Result<()> {
        print_debug!("app change - {app}");
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
        print_debug!("handle button - {id} {button:?} {phase:?}");
        let active_app = self.get_active_app();
        let Some(app_rules) = self.profile.app_rules.get(&active_app).cloned()
        else {
            return vec![];
        };
        let state_ref = self
            .controllers
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
