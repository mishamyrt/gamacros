use std::sync::Arc;

use ahash::AHashMap;

use gamacros_gamepad::{Axis as CtrlAxis, ControllerId};
use gamacros_control::{Key, KeyCombo};
use gamacros_profile::{
    ArrowsParams, Axis as ProfileAxis, MouseParams, ScrollParams, StepperParams,
    StickMode, StickRules, StickSide,
};

use super::gamacros::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct StickProcessor {
    controllers: AHashMap<ControllerId, ControllerRepeatState>,
    generation: u64,
    // Reusable scratch buffer to avoid per-tick allocations
    regs: Vec<RepeatReg>,
}

#[derive(Default)]
struct ControllerRepeatState {
    sides: [SideRepeatState; 2],
}

#[derive(Default)]
struct SideRepeatState {
    scroll_accum: (f32, f32),
    arrows: [Option<RepeatTaskState>; 4],
    volume: [Option<RepeatTaskState>; 4],
    brightness: [Option<RepeatTaskState>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RepeatKind {
    Arrow(Direction),
    Volume { axis: ProfileAxis, positive: bool },
    Brightness { axis: ProfileAxis, positive: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RepeatTaskId {
    controller: ControllerId,
    side: StickSide,
    kind: RepeatKind,
}

struct RepeatTaskState {
    key: Key,
    fire_on_activate: bool,
    initial_delay_ms: u64,
    interval_ms: u64,
    last_fire: std::time::Instant,
    delay_done: bool,
    last_seen_generation: u64,
}

struct RepeatReg {
    id: RepeatTaskId,
    key: Key,
    fire_on_activate: bool,
    initial_delay_ms: u64,
    interval_ms: u64,
}

enum StepperMode {
    Volume,
    Brightness,
}

impl StepperMode {
    fn key_for(&self, positive: bool) -> Key {
        match self {
            StepperMode::Volume => {
                if positive {
                    Key::VolumeUp
                } else {
                    Key::VolumeDown
                }
            }
            StepperMode::Brightness => {
                if positive {
                    Key::BrightnessUp
                } else {
                    Key::BrightnessDown
                }
            }
        }
    }

    fn kind_for(&self, axis: ProfileAxis, positive: bool) -> RepeatKind {
        match self {
            StepperMode::Volume => RepeatKind::Volume { axis, positive },
            StepperMode::Brightness => RepeatKind::Brightness { axis, positive },
        }
    }
}

impl StickProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn axis_index(axis: CtrlAxis) -> usize {
        match axis {
            CtrlAxis::LeftX => 0,
            CtrlAxis::LeftY => 1,
            CtrlAxis::RightX => 2,
            CtrlAxis::RightY => 3,
            CtrlAxis::LeftTrigger => 4,
            CtrlAxis::RightTrigger => 5,
        }
    }

    fn side_index(side: &StickSide) -> usize {
        match side {
            StickSide::Left => 0,
            StickSide::Right => 1,
        }
    }

    fn dir_index(dir: Direction) -> usize {
        match dir {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        }
    }

    fn step_slot_index(axis: ProfileAxis, positive: bool) -> usize {
        match (axis, positive) {
            (ProfileAxis::X, false) => 0,
            (ProfileAxis::X, true) => 1,
            (ProfileAxis::Y, false) => 2,
            (ProfileAxis::Y, true) => 3,
        }
    }

    pub fn release_all_for(&mut self, id: ControllerId) {
        self.controllers.remove(&id);
    }

    pub fn release_all_arrows(&mut self) {
        for (_cid, state) in self.controllers.iter_mut() {
            for s in 0..2 {
                for slot in state.sides[s].arrows.iter_mut() {
                    *slot = None;
                }
            }
        }
    }

    pub fn on_app_change(&mut self) {
        self.release_all_arrows();
        for (_cid, state) in self.controllers.iter_mut() {
            for s in 0..2 {
                state.sides[s].scroll_accum = (0.0, 0.0);
            }
        }
    }

    pub fn axes_for_side(axes: [f32; 6], side: &StickSide) -> (f32, f32) {
        match side {
            StickSide::Left => (
                axes[Self::axis_index(CtrlAxis::LeftX)],
                axes[Self::axis_index(CtrlAxis::LeftY)],
            ),
            StickSide::Right => (
                axes[Self::axis_index(CtrlAxis::RightX)],
                axes[Self::axis_index(CtrlAxis::RightY)],
            ),
        }
    }

    pub fn invert_xy(x: f32, y: f32, invert_x: bool, invert_y: bool) -> (f32, f32) {
        let nx = if invert_x { -x } else { x };
        let ny = if invert_y { -y } else { y };
        (nx, ny)
    }

    pub fn magnitude2d(x: f32, y: f32) -> f32 {
        (x * x + y * y).sqrt()
    }

    pub fn normalize_after_deadzone(mag: f32, deadzone: f32) -> f32 {
        if mag <= deadzone {
            0.0
        } else {
            ((mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0)
        }
    }

    pub fn quantize_direction(x: f32, y: f32) -> Option<Direction> {
        let ax = x.abs();
        let ay = y.abs();
        if ax == 0.0 && ay == 0.0 {
            return None;
        }
        if ax > ay {
            if x > 0.0 {
                Some(Direction::Right)
            } else {
                Some(Direction::Left)
            }
        } else if ay > ax {
            if y > 0.0 {
                Some(Direction::Up)
            } else {
                Some(Direction::Down)
            }
        } else if y > 0.0 {
            Some(Direction::Up)
        } else if y < 0.0 {
            Some(Direction::Down)
        } else {
            None
        }
    }

    pub fn get_direction_key(dir: Direction) -> Key {
        match dir {
            Direction::Up => Key::UpArrow,
            Direction::Down => Key::DownArrow,
            Direction::Left => Key::LeftArrow,
            Direction::Right => Key::RightArrow,
        }
    }

    pub fn on_tick_with<F: FnMut(Action)>(
        &mut self,
        bindings: Option<&StickRules>,
        axes_list: &[(ControllerId, [f32; 6])],
        mut sink: F,
    ) {
        // Early exit when idle
        if axes_list.is_empty() && !self.has_active_repeats() {
            return;
        }
        let Some(bindings) = bindings else {
            return;
        };

        let mut arrow_bindings: Vec<(&StickSide, &ArrowsParams)> = Vec::new();
        let mut volume_bindings: Vec<(&StickSide, &StepperParams)> = Vec::new();
        let mut brightness_bindings: Vec<(&StickSide, &StepperParams)> = Vec::new();
        let mut mouse_bindings: Vec<(&StickSide, &MouseParams)> = Vec::new();
        let mut scroll_bindings: Vec<(&StickSide, &ScrollParams)> = Vec::new();
        for (side, mode) in bindings.iter() {
            match mode {
                StickMode::Arrows(params) => arrow_bindings.push((side, params)),
                StickMode::Volume(params) => volume_bindings.push((side, params)),
                StickMode::Brightness(params) => {
                    brightness_bindings.push((side, params))
                }
                StickMode::MouseMove(params) => mouse_bindings.push((side, params)),
                StickMode::Scroll(params) => scroll_bindings.push((side, params)),
            }
        }

        let now = std::time::Instant::now();
        // bump generation for this tick
        self.generation = self.generation.wrapping_add(1);

        if !arrow_bindings.is_empty() {
            self.tick_arrows(now, &mut sink, axes_list, &arrow_bindings);
        }
        if !volume_bindings.is_empty() {
            self.tick_volume(now, &mut sink, axes_list, &volume_bindings);
        }
        if !brightness_bindings.is_empty() {
            self.tick_brightness(now, &mut sink, axes_list, &brightness_bindings);
        }
        if !mouse_bindings.is_empty() {
            self.tick_mouse(&mut sink, axes_list, &mouse_bindings);
        }
        if !scroll_bindings.is_empty() {
            self.tick_scroll(&mut sink, axes_list, &scroll_bindings);
        }

        self.repeater_drain_due(now, &mut sink);
        self.repeater_cleanup_inactive();
    }

    fn has_active_repeats(&self) -> bool {
        for (_cid, ctrl) in self.controllers.iter() {
            for side in ctrl.sides.iter() {
                if side.arrows.iter().any(|s| s.is_some())
                    || side.volume.iter().any(|s| s.is_some())
                    || side.brightness.iter().any(|s| s.is_some())
                {
                    return true;
                }
            }
        }
        false
    }

    fn tick_arrows(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        arrow_bindings: &[(&StickSide, &ArrowsParams)],
    ) {
        self.regs.clear();
        for (id, axes) in axes_list.iter().cloned() {
            for (side, params) in arrow_bindings.iter() {
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (x, y) =
                    Self::invert_xy(x0, y0, params.invert_x, !params.invert_y);
                let mag2 = x * x + y * y;
                let dead2 = params.deadzone * params.deadzone;
                let new_dir = if mag2 < dead2 {
                    None
                } else {
                    Self::quantize_direction(x, y)
                };
                if let Some(dir) = new_dir {
                    let task_id = RepeatTaskId {
                        controller: id,
                        side: **side,
                        kind: RepeatKind::Arrow(dir),
                    };
                    let key = Self::get_direction_key(dir);
                    self.regs.push(RepeatReg {
                        id: task_id,
                        key,
                        fire_on_activate: true,
                        initial_delay_ms: params.repeat_delay_ms,
                        interval_ms: params.repeat_interval_ms,
                    });
                }
            }
        }
        let regs = std::mem::take(&mut self.regs);
        for reg in regs {
            if let Some(a) = self.repeater_register(reg, now) {
                (sink)(a);
            }
        }
    }

    fn tick_volume(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &[(&StickSide, &StepperParams)],
    ) {
        self.tick_stepper(now, sink, axes_list, bindings, StepperMode::Volume);
    }

    fn tick_brightness(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &[(&StickSide, &StepperParams)],
    ) {
        self.tick_stepper(now, sink, axes_list, bindings, StepperMode::Brightness);
    }

    fn tick_stepper(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &[(&StickSide, &StepperParams)],
        mode: StepperMode,
    ) {
        self.regs.clear();
        for (side, params) in bindings.iter() {
            for (cid, axes) in axes_list.iter().cloned() {
                let (x, y) = (
                    axes[Self::axis_index(CtrlAxis::LeftX)],
                    axes[Self::axis_index(CtrlAxis::LeftY)],
                );
                let (rx, ry) = (
                    axes[Self::axis_index(CtrlAxis::RightX)],
                    axes[Self::axis_index(CtrlAxis::RightY)],
                );
                let (vx, vy) = match side {
                    StickSide::Left => (x, y),
                    StickSide::Right => (rx, ry),
                };
                let v = match params.axis {
                    ProfileAxis::X => vx,
                    ProfileAxis::Y => vy,
                };
                let mag = v.abs();
                if mag < params.deadzone {
                    continue;
                }
                let t = mag;
                let interval_ms = (params.max_interval_ms as f32)
                    + (1.0 - t)
                        * ((params.min_interval_ms as f32)
                            - (params.max_interval_ms as f32));
                let positive = v >= 0.0;
                let key = mode.key_for(positive);
                let kind = mode.kind_for(params.axis, positive);
                let task_id = RepeatTaskId {
                    controller: cid,
                    side: **side,
                    kind,
                };
                self.regs.push(RepeatReg {
                    id: task_id,
                    key,
                    fire_on_activate: true,
                    initial_delay_ms: 0,
                    interval_ms: interval_ms as u64,
                });
            }
        }
        let regs = std::mem::take(&mut self.regs);
        for reg in regs {
            if let Some(a) = self.repeater_register(reg, now) {
                (sink)(a);
            }
        }
    }

    fn repeater_register(
        &mut self,
        reg: RepeatReg,
        now: std::time::Instant,
    ) -> Option<Action> {
        let cid = reg.id.controller;
        let side_idx = Self::side_index(&reg.id.side);
        let ctrl = self.controllers.entry(cid).or_default();
        let side = &mut ctrl.sides[side_idx];

        let slot: &mut Option<RepeatTaskState> = match reg.id.kind {
            RepeatKind::Arrow(dir) => {
                let idx = Self::dir_index(dir);
                &mut side.arrows[idx]
            }
            RepeatKind::Volume { axis, positive } => {
                let idx = Self::step_slot_index(axis, positive);
                &mut side.volume[idx]
            }
            RepeatKind::Brightness { axis, positive } => {
                let idx = Self::step_slot_index(axis, positive);
                &mut side.brightness[idx]
            }
        };

        match slot {
            Some(st) => {
                st.key = reg.key;
                st.interval_ms = reg.interval_ms;
                st.initial_delay_ms = reg.initial_delay_ms;
                st.fire_on_activate = reg.fire_on_activate;
                st.last_seen_generation = self.generation;
                None
            }
            None => {
                let st = RepeatTaskState {
                    key: reg.key,
                    fire_on_activate: reg.fire_on_activate,
                    initial_delay_ms: reg.initial_delay_ms,
                    interval_ms: reg.interval_ms,
                    last_fire: now,
                    delay_done: reg.initial_delay_ms == 0,
                    last_seen_generation: self.generation,
                };
                *slot = Some(st);
                if reg.fire_on_activate {
                    let key = reg.key.to_owned();
                    Some(Action::KeyTap(Arc::new(KeyCombo::from_key(key))))
                } else {
                    None
                }
            }
        }
    }

    fn repeater_drain_due(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
    ) {
        for (_cid, ctrl) in self.controllers.iter_mut() {
            for side in ctrl.sides.iter_mut() {
                for slot in side
                    .arrows
                    .iter_mut()
                    .chain(side.volume.iter_mut())
                    .chain(side.brightness.iter_mut())
                {
                    if let Some(st) = slot.as_mut() {
                        let due_ms = if st.delay_done {
                            st.interval_ms
                        } else {
                            st.initial_delay_ms
                        };
                        if due_ms == 0 {
                            continue;
                        }
                        let elapsed =
                            now.duration_since(st.last_fire).as_millis() as u64;
                        if elapsed >= due_ms {
                            (sink)(Action::KeyTap(Arc::new(KeyCombo::from_key(
                                st.key,
                            ))));
                            st.last_fire = now;
                            st.delay_done = true;
                        }
                    }
                }
            }
        }
    }

    fn repeater_cleanup_inactive(&mut self) {
        let gen = self.generation;
        for (_cid, ctrl) in self.controllers.iter_mut() {
            for side in ctrl.sides.iter_mut() {
                for slot in side.arrows.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
                for slot in side.volume.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
                for slot in side.brightness.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
            }
        }
    }

    fn tick_mouse(
        &mut self,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &[(&StickSide, &MouseParams)],
    ) {
        for (side, params) in bindings.iter() {
            for (_cid, axes) in axes_list.iter().cloned() {
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (x, y) =
                    Self::invert_xy(x0, y0, params.invert_x, params.invert_y);
                let mag_raw = Self::magnitude2d(x, y);
                if mag_raw < params.deadzone {
                    continue;
                }
                let base = Self::normalize_after_deadzone(mag_raw, params.deadzone);
                let gamma = params.gamma.max(0.1);
                let mag = if (gamma - 1.0).abs() < 1e-6 {
                    base
                } else if (gamma - 2.0).abs() < 1e-6 {
                    base * base
                } else {
                    base.powf(gamma)
                };
                if mag <= 0.0 {
                    continue;
                }
                let dir_x = x / mag_raw;
                let dir_y = y / mag_raw;
                let speed_px_s = params.max_speed_px_s * mag;
                let dt_s = 0.010;
                let dx = (speed_px_s * dir_x * dt_s).round() as i32;
                let dy = (speed_px_s * dir_y * dt_s).round() as i32;
                if dx != 0 || dy != 0 {
                    (sink)(Action::MouseMove { dx, dy });
                }
            }
        }
    }

    fn tick_scroll(
        &mut self,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &[(&StickSide, &ScrollParams)],
    ) {
        for (side, params) in bindings.iter() {
            for (cid, axes) in axes_list.iter().cloned() {
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (mut x, y) =
                    Self::invert_xy(x0, y0, params.invert_x, !params.invert_y);
                if !params.horizontal {
                    x = 0.0;
                }
                let mag_raw = x.abs().max(y.abs());
                if mag_raw <= params.deadzone {
                    continue;
                }
                let dt_s = 0.1;
                let sidx = Self::side_index(side);
                let accum = &mut self.controllers.entry(cid).or_default().sides
                    [sidx]
                    .scroll_accum;
                accum.0 += params.speed_lines_s * x * dt_s;
                accum.1 += params.speed_lines_s * y * dt_s;
                let h = accum.0.round() as i32;
                let v = accum.1.round() as i32;
                if h != 0 {
                    (sink)(Action::Scroll { h, v: 0 });
                    accum.0 -= h as f32;
                }
                if v != 0 {
                    (sink)(Action::Scroll { h: 0, v });
                    accum.1 -= v as f32;
                }
            }
        }
    }
}
