use std::cell::RefCell;
use std::sync::Arc;
use ahash::AHashMap;

use colored::Colorize;

use gamacros_control::KeyCombo;
use gamacros_bit_mask::Bitmask;
use gamacros_gamepad::{Button, ControllerId, ControllerInfo, Axis as CtrlAxis};
use gamacros_workspace::{
    ButtonAction, ButtonRule, ControllerSettings, Profile, StickRules,
};

use crate::{app::ButtonPhase, print_debug, print_info};
use super::stick::{StickProcessor, CompiledStickRules};
use super::stick::util::axis_index as stick_axis_index;

#[derive(Debug, Clone)]
pub enum Action {
    KeyPress(Arc<KeyCombo>),
    KeyRelease(Arc<KeyCombo>),
    KeyTap(Arc<KeyCombo>),
    Macros(Arc<Vec<KeyCombo>>),
    Shell(String),
    MouseMove { dx: i32, dy: i32 },
    Scroll { h: i32, v: i32 },
    Rumble { id: ControllerId, ms: u32 },
}

#[derive(Debug)]
struct ControllerState {
    mapping: ControllerSettings,
    pressed: Bitmask<Button>,
    rumble: bool,
    axes: [f32; 6],
}

pub struct Gamacros {
    pub workspace: Option<Profile>,
    active_app: Box<str>,
    controllers: AHashMap<ControllerId, ControllerState>,
    sticks: RefCell<StickProcessor>,
    active_stick_rules: Option<Arc<StickRules>>, // keep original for potential future use
    compiled_stick_rules: Option<CompiledStickRules>,
}

impl Gamacros {
    pub fn new() -> Self {
        Self {
            workspace: None,
            active_app: "".into(),
            controllers: AHashMap::new(),
            sticks: RefCell::new(StickProcessor::new()),
            active_stick_rules: None,
            compiled_stick_rules: None,
        }
    }

    pub fn is_known(&self, id: ControllerId) -> bool {
        self.controllers.contains_key(&id)
    }

    pub fn remove_workspace(&mut self) {
        self.workspace = None;
        self.active_stick_rules = None;
        self.compiled_stick_rules = None;
    }

    pub fn set_workspace(&mut self, workspace: Profile) {
        self.workspace = Some(workspace);
        // Recompute stick rules for current active app (workspace may have changed)
        if !self.active_app.is_empty() {
            if let Some(ws) = self.workspace.as_ref() {
                if let Some(app_rules) = ws.rules.get(&*self.active_app) {
                    self.active_stick_rules =
                        Some(Arc::new(app_rules.sticks.clone()));
                    self.compiled_stick_rules = self
                        .active_stick_rules
                        .as_deref()
                        .map(CompiledStickRules::from_rules);
                } else {
                    self.active_stick_rules = None;
                    self.compiled_stick_rules = None;
                }
            }
        }
    }

    pub fn add_controller(&mut self, info: ControllerInfo) {
        print_info!(
            "add controller - {0} id={1} vid=0x{2:x} pid=0x{3:x}",
            info.name,
            info.id,
            info.vendor_id,
            info.product_id
        );

        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        let settings = workspace
            .controllers
            .get(&(info.vendor_id, info.product_id))
            .cloned();
        let state = ControllerState {
            mapping: settings.unwrap_or_default(),
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
        if self.active_app.as_ref() == app {
            return;
        }
        if self.active_app.as_ref() == "" {
            print_debug!("got active app - {app}");
        } else {
            print_debug!("app change - {app}");
        }

        self.active_app = app.into();
        self.sticks.borrow_mut().on_app_change();
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };

        self.active_stick_rules = workspace
            .rules
            .get(&*self.active_app)
            .map(|r| Arc::new(r.sticks.clone()));

        self.compiled_stick_rules = self
            .active_stick_rules
            .as_deref()
            .map(CompiledStickRules::from_rules);
    }

    pub fn get_active_app(&self) -> &str {
        &self.active_app
    }

    pub fn get_compiled_stick_rules(&self) -> Option<&CompiledStickRules> {
        self.compiled_stick_rules.as_ref()
    }

    pub fn on_axis_motion(&mut self, id: ControllerId, axis: CtrlAxis, value: f32) {
        let idx = stick_axis_index(axis);
        if let Some(st) = self.controllers.get_mut(&id) {
            st.axes[idx] = value;
        }
    }

    pub fn on_controller_disconnected(&mut self, id: ControllerId) {
        self.sticks.borrow_mut().release_all_for(id);
    }

    pub fn on_tick_with<F: FnMut(Action)>(&mut self, sink: F) {
        let bindings_ref = self.get_compiled_stick_rules();
        // Avoid borrowing self while calling into sticks; use a local buffer
        let mut axes_scratch: Vec<(ControllerId, [f32; 6])> =
            Vec::with_capacity(self.controllers.len());
        for (id, st) in self.controllers.iter() {
            axes_scratch.push((*id, st.axes));
        }
        self.sticks
            .borrow_mut()
            .on_tick_with(bindings_ref, &axes_scratch, sink);
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
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        let Some(app_rules) = workspace.rules.get(active_app) else {
            return;
        };
        let state = self
            .controllers
            .get_mut(&id)
            .expect("device must be added before use");
        let button = state.mapping.mapping.get(&button).unwrap_or(&button);

        // snapshot before change
        let prev_pressed = state.pressed;

        if phase == ButtonPhase::Pressed {
            state.pressed.insert(*button);
        } else {
            state.pressed.remove(*button);
        }

        // snapshot after change
        let now_pressed = state.pressed;

        let mut candidates: Vec<(&ButtonRule, u32)> = vec![];

        for (target, rule) in app_rules.buttons.iter() {
            let was = prev_pressed.is_superset(target);
            let is_now = now_pressed.is_superset(target);

            let fire = match phase {
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
            match phase {
                ButtonPhase::Pressed => {
                    if let Some(ms) = rule.vibrate {
                        if self.supports_rumble(id) {
                            sink(Action::Rumble { id, ms: ms as u32 });
                        }
                    }
                    match rule.action.clone() {
                        ButtonAction::Keystroke(k) => {
                            sink(Action::KeyPress(k));
                        }
                        ButtonAction::Macros(m) => {
                            sink(Action::Macros(m));
                        }
                        ButtonAction::Shell(s) => {
                            print_debug!("shell command: {}", s);
                            sink(Action::Shell(s));
                        }
                    }
                }
                ButtonPhase::Released => {
                    if let ButtonAction::Keystroke(k) = rule.action.clone() {
                        sink(Action::KeyRelease(k));
                    }
                }
            }
        }
    }
}
