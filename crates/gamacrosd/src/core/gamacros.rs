use std::cell::RefCell;
use std::sync::Arc;
use std::collections::HashMap;

use colored::Colorize;

use gamacros_control::KeyCombo;
use gamacros_bit_mask::Bitmask;
use gamacros_gamepad::{Button, ControllerId, ControllerInfo, Axis as CtrlAxis};
use gamacros_profile::{ButtonPhase, ButtonRule, Profile, StickRules};

use crate::{print_debug, print_info};
use super::stick::StickProcessor;

#[derive(Debug, Clone)]
pub enum Action {
    KeyPress(Arc<KeyCombo>),
    KeyRelease(Arc<KeyCombo>),
    KeyTap(Arc<KeyCombo>),
    MouseMove { dx: i32, dy: i32 },
    Scroll { h: i32, v: i32 },
    Rumble { id: ControllerId, ms: u32 },
}

const BUTTON_COUNT: usize = gamacros_gamepad::Button::DPadRight as usize + 1;

#[derive(Debug, Clone)]
pub struct ControllerMapping([Option<Button>; BUTTON_COUNT]);

impl Default for ControllerMapping {
    fn default() -> Self {
        Self([None; BUTTON_COUNT])
    }
}

impl ControllerMapping {
    pub fn new(mapping: HashMap<Button, Button>) -> Self {
        let mut arr: [Option<Button>; BUTTON_COUNT] = [None; BUTTON_COUNT];
        for (from, to) in mapping.into_iter() {
            let idx = from as usize;
            if idx < BUTTON_COUNT {
                arr[idx] = Some(to);
            }
        }
        Self(arr)
    }

    pub fn get(&self, button: Button) -> Button {
        let idx = button as usize;
        if idx < BUTTON_COUNT {
            self.0[idx].unwrap_or(button)
        } else {
            button
        }
    }
}

#[derive(Debug)]
struct ControllerState {
    mapping: ControllerMapping,
    pressed: Bitmask<Button>,
    rumble: bool,
    axes: [f32; 6],
}

pub struct Gamacros {
    profile: Profile,
    active_app: Box<str>,
    controllers: HashMap<ControllerId, ControllerState>,
    sticks: RefCell<StickProcessor>,
    active_stick_rules: Option<Arc<StickRules>>,
    axes_scratch: Vec<(ControllerId, [f32; 6])>,
}

impl Gamacros {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            active_app: "".into(),
            controllers: HashMap::new(),
            sticks: RefCell::new(StickProcessor::new()),
            active_stick_rules: None,
            axes_scratch: Vec::new(),
        }
    }

    pub fn is_known(&self, id: ControllerId) -> bool {
        self.controllers.contains_key(&id)
    }

    pub fn add_controller(&mut self, info: ControllerInfo) {
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
            pressed: Bitmask::empty(),
            rumble: info.supports_rumble,
            axes: [0.0; 6],
        };
        if self.is_known(info.id) {
            print_debug!("controller already known - id={0}", info.id);
        }
        self.controllers.insert(info.id, state);
    }

    pub fn remove_controller(&mut self, id: ControllerId) {
        print_info!("remove device - {id:x}");
        self.controllers.remove(&id);
    }

    pub fn supports_rumble(&self, id: ControllerId) -> bool {
        self.controllers.get(&id).map(|s| s.rumble).unwrap_or(false)
    }

    pub fn set_active_app(&mut self, app: &str) {
        print_debug!("app change - {app}");
        self.active_app = app.into();
        self.sticks.borrow_mut().on_app_change();
        self.active_stick_rules = self
            .profile
            .rules
            .get(&*self.active_app)
            .map(|r| Arc::new(r.sticks.clone()));
    }

    pub fn get_active_app(&self) -> &str {
        &self.active_app
    }

    pub fn get_stick_bindings_arc(&self) -> Option<Arc<StickRules>> {
        self.active_stick_rules.clone()
    }

    pub fn on_axis_motion(&mut self, id: ControllerId, axis: CtrlAxis, value: f32) {
        let idx = StickProcessor::axis_index(axis);
        if let Some(st) = self.controllers.get_mut(&id) {
            st.axes[idx] = value;
        }
    }

    pub fn on_controller_disconnected(&mut self, id: ControllerId) {
        self.sticks.borrow_mut().release_all_for(id);
    }

    pub fn on_tick_with<F: FnMut(Action)>(&mut self, sink: F) {
        let bindings_arc = self.get_stick_bindings_arc();
        let bindings_ref = bindings_arc.as_deref();
        self.axes_scratch.clear();
        self.axes_scratch.reserve(self.controllers.len());
        for (id, st) in self.controllers.iter() {
            self.axes_scratch.push((*id, st.axes));
        }
        self.sticks.borrow_mut().on_tick_with(
            bindings_ref,
            &self.axes_scratch,
            sink,
        );
    }

    pub fn on_button_with<F: FnMut(Action)>(
        &mut self,
        id: ControllerId,
        button: Button,
        phase: ButtonPhase,
        mut sink: F,
    ) {
        print_debug!("handle button - {id} {button:?} {phase:?}");
        let active_app = self.get_active_app();
        let Some(app_rules) = self.profile.rules.get(active_app) else {
            return;
        };
        let state = self
            .controllers
            .get_mut(&id)
            .expect("device must be added before use");
        let button = state.mapping.get(button);

        // snapshot before change
        let prev_pressed = state.pressed;

        if phase == ButtonPhase::Pressed {
            state.pressed.insert(button);
        } else {
            state.pressed.remove(button);
        }

        // snapshot after change
        let now_pressed = state.pressed;

        let mut candidates: Vec<(&ButtonRule, u32)> = vec![];

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
                candidates.push((rule, bits));
            }
        }

        if candidates.is_empty() {
            return;
        }

        let max_bits = candidates.iter().map(|(_, b)| *b).max().unwrap_or(0);
        for (rule, bits) in candidates.into_iter() {
            if bits != max_bits {
                continue;
            }
            match rule.when {
                ButtonPhase::Pressed => match phase {
                    ButtonPhase::Pressed => {
                        if let Some(ms) = rule.vibrate {
                            if self.supports_rumble(id) {
                                sink(Action::Rumble { id, ms: ms as u32 });
                            }
                        }
                        sink(Action::KeyPress(rule.action.clone()));
                    }
                    ButtonPhase::Released => {
                        sink(Action::KeyRelease(rule.action.clone()));
                    }
                },
                ButtonPhase::Released => {
                    if let Some(ms) = rule.vibrate {
                        if self.supports_rumble(id) {
                            sink(Action::Rumble { id, ms: ms as u32 });
                        }
                    }
                    sink(Action::KeyTap(rule.action.clone()));
                }
            }
        }
    }
}
