use std::{
    collections::HashMap,
    sync::{RwLock, Mutex},
};

use dashmap::DashMap;
use thiserror::Error;
use colored::Colorize;

use crate::{print_debug, print_info, stick::StickProcessor};

use gamacros_keypress::KeyCombo;
use gamacros_bit_mask::AtomicBitmask;
use gamacros_controller::{Button, ControllerId, ControllerInfo, Axis as CtrlAxis};
use gamacros_profile::{ButtonPhase, ButtonRule, Profile, StickRules};

#[derive(Debug, Clone)]
pub enum Action {
    KeyPress(KeyCombo),
    KeyRelease(KeyCombo),
    KeyTap(KeyCombo),
    MouseMove { dx: i32, dy: i32 },
    Scroll { h: i32, v: i32 },
    Rumble { id: ControllerId, ms: u32 },
}

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
    sticks: Mutex<StickProcessor>,
}

impl Gamacros {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            active_app: RwLock::new("".into()),
            controllers: DashMap::new(),
            sticks: Mutex::new(StickProcessor::new()),
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
            .controllers
            .iter()
            .find(|d| d.vid == info.vendor_id && d.pid == info.product_id)
            .map(|d| ControllerMapping::new(d.remap.clone()))
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
                if let Ok(mut sticks) = self.sticks.lock() {
                    sticks.on_app_change();
                }
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

    pub fn get_stick_bindings(&self) -> Option<StickRules> {
        let active_app = self.get_active_app();
        self.profile
            .rules
            .get(&active_app)
            .map(|r| r.sticks.clone())
    }

    pub fn on_axis_motion(&self, id: ControllerId, axis: CtrlAxis, value: f32) {
        if let Ok(mut s) = self.sticks.lock() {
            s.update_axis(id, axis, value);
        }
    }

    pub fn on_controller_disconnected(&self, id: ControllerId) {
        if let Ok(mut s) = self.sticks.lock() {
            s.release_all_for(id);
        }
    }

    pub fn on_tick(&self) -> Vec<Action> {
        let bindings = self.get_stick_bindings();
        match self.sticks.lock() {
            Ok(mut s) => s.on_tick(&bindings),
            Err(_) => vec![],
        }
    }

    pub fn on_button(
        &self,
        id: ControllerId,
        button: Button,
        phase: ButtonPhase,
    ) -> Vec<Action> {
        print_debug!("handle button - {id} {button:?} {phase:?}");
        let active_app = self.get_active_app();
        let Some(app_rules) = self.profile.rules.get(&active_app).cloned() else {
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

        if phase == ButtonPhase::Pressed {
            state.pressed.insert(button);
        } else {
            state.pressed.remove(button);
        }

        // snapshot after change
        let now_pressed = state.pressed.load();

        let mut candidates: Vec<(ButtonRule, u32)> = vec![];

        for (target, rule) in app_rules.buttons.iter() {
            let was = prev_pressed.is_superset(target);
            let is_now = now_pressed.is_superset(target);

            let fire = match rule.when {
                // For Pressed rules, fire on both edges (activation and deactivation)
                // so that main.rs can press on activation (phase=Pressed)
                // and release on deactivation (phase=Released).
                ButtonPhase::Pressed => was != is_now,
                ButtonPhase::Released => was && !is_now,
            };

            if fire {
                let bits: u32 = target.count();
                candidates.push((rule.clone(), bits));
            }
        }

        if candidates.is_empty() {
            return vec![];
        }

        let max_bits = candidates.iter().map(|(_, b)| *b).max().unwrap_or(0);
        let mut actions: Vec<Action> = Vec::new();
        for (rule, bits) in candidates.into_iter() {
            if bits != max_bits {
                continue;
            }
            match rule.when {
                ButtonPhase::Pressed => match phase {
                    ButtonPhase::Pressed => {
                        if let Some(ms) = rule.vibrate {
                            if self.supports_rumble(id) {
                                actions.push(Action::Rumble { id, ms: ms as u32 });
                            }
                        }
                        actions.push(Action::KeyPress(rule.action.clone()));
                    }
                    ButtonPhase::Released => {
                        actions.push(Action::KeyRelease(rule.action.clone()));
                    }
                },
                ButtonPhase::Released => {
                    if let Some(ms) = rule.vibrate {
                        if self.supports_rumble(id) {
                            actions.push(Action::Rumble { id, ms: ms as u32 });
                        }
                    }
                    actions.push(Action::KeyTap(rule.action.clone()));
                }
            }
        }

        actions
    }
}
