use ahash::AHashMap;

use enigo::Key;
use gamacros_gamepad::{Axis as CtrlAxis, ControllerId};
use gamacros_control::KeyCombo;
use gamacros_profile::{ArrowsParams, Axis as ProfileAxis, MouseParams, ScrollParams, StepperParams, StickMode, StickRules, StickSide};

use crate::gamacros::Action;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct StickProcessor {
    axes: AHashMap<ControllerId, [f32; 6]>,
    scroll_accum: AHashMap<(ControllerId, StickSide), (f32, f32)>,
    repeat_tasks: AHashMap<RepeatTaskId, RepeatTaskState>,
    generation: u64,
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

enum StepperMode { Volume, Brightness }

impl StepperMode {
    fn key_for(&self, positive: bool) -> Key {
        match self {
            StepperMode::Volume => {
                if positive { Key::VolumeUp } else { Key::VolumeDown }
            }
            StepperMode::Brightness => {
                if positive { Key::BrightnessUp } else { Key::BrightnessDown }
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

    pub fn update_axis(&mut self, id: ControllerId, axis: CtrlAxis, value: f32) {
        let idx = Self::axis_index(axis);
        let entry = self.axes.entry(id).or_insert([0.0; 6]);
        entry[idx] = value;
    }

    pub fn release_all_for(&mut self, id: ControllerId) {
        self.scroll_accum.retain(|(cid, _), _| *cid != id);
        self.repeat_tasks.retain(|task_id, _| task_id.controller != id);
        self.axes.remove(&id);
    }

    pub fn release_all_arrows(&mut self) {
        self.repeat_tasks
            .retain(|task_id, _| matches!(task_id.kind, RepeatKind::Volume { .. } | RepeatKind::Brightness { .. }));
    }

    pub fn on_app_change(&mut self) {
        self.release_all_arrows();
        self.scroll_accum.clear();
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

    pub fn on_tick(&mut self, bindings: Option<&StickRules>) -> Vec<Action> {
        let mut out: Vec<Action> = Vec::new();
        // Early exit when idle
        if self.axes.is_empty() && self.repeat_tasks.is_empty() {
            return out;
        }
        let Some(bindings) = bindings else { return out; };

        let mut arrow_bindings: Vec<(&StickSide, &ArrowsParams)> = Vec::new();
        let mut volume_bindings: Vec<(&StickSide, &StepperParams)> = Vec::new();
        let mut brightness_bindings: Vec<(&StickSide, &StepperParams)> = Vec::new();
        let mut mouse_bindings: Vec<(&StickSide, &MouseParams)> = Vec::new();
        let mut scroll_bindings: Vec<(&StickSide, &ScrollParams)> = Vec::new();
        for (side, mode) in bindings.iter() {
            match mode {
                StickMode::Arrows(params) => arrow_bindings.push((side, params)),
                StickMode::Volume(params) => volume_bindings.push((side, params)),
                StickMode::Brightness(params) => brightness_bindings.push((side, params)),
                StickMode::MouseMove(params) => mouse_bindings.push((side, params)),
                StickMode::Scroll(params) => scroll_bindings.push((side, params)),
            }
        }

        let now = std::time::Instant::now();
        // bump generation for this tick
        self.generation = self.generation.wrapping_add(1);

        if !arrow_bindings.is_empty() {
            self.tick_arrows(now, &mut out, &arrow_bindings);
        }
        if !volume_bindings.is_empty() {
            self.tick_volume(now, &mut out, &volume_bindings);
        }
        if !brightness_bindings.is_empty() {
            self.tick_brightness(now, &mut out, &brightness_bindings);
        }
        if !mouse_bindings.is_empty() {
            self.tick_mouse(&mut out, &mouse_bindings);
        }
        if !scroll_bindings.is_empty() {
            self.tick_scroll(&mut out, &scroll_bindings);
        }

        self.repeater_drain_due(now, &mut out);
        self.repeater_cleanup_inactive();

        out
    }

    fn tick_arrows(&mut self, now: std::time::Instant, out: &mut Vec<Action>, arrow_bindings: &[(&StickSide, &ArrowsParams)]) {
        let mut regs: Vec<RepeatReg> = Vec::new();
        for (id, axes_arr) in self.axes.iter() {
            let id = *id;
            let axes = *axes_arr;
            for (side, params) in arrow_bindings.iter() {
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (x, y) = Self::invert_xy(x0, y0, params.invert_x, !params.invert_y);
                let mag2 = x * x + y * y;
                let dead2 = params.deadzone * params.deadzone;
                let new_dir = if mag2 < dead2 { None } else { Self::quantize_direction(x, y) };
                if let Some(dir) = new_dir {
                    let task_id = RepeatTaskId { controller: id, side: **side, kind: RepeatKind::Arrow(dir) };
                    let key = Self::get_direction_key(dir);
                    regs.push(RepeatReg { id: task_id, key, fire_on_activate: true, initial_delay_ms: params.repeat_delay_ms, interval_ms: params.repeat_interval_ms });
                }
            }
        }
        for reg in regs.into_iter() {
            if let Some(a) = self.repeater_register(reg, now) { out.push(a); }
        }
    }

    fn tick_volume(&mut self, now: std::time::Instant, out: &mut Vec<Action>, bindings: &[(&StickSide, &StepperParams)]) {
        self.tick_stepper(now, out, bindings, StepperMode::Volume);
    }

    fn tick_brightness(&mut self, now: std::time::Instant, out: &mut Vec<Action>, bindings: &[(&StickSide, &StepperParams)]) {
        self.tick_stepper(now, out, bindings, StepperMode::Brightness);
    }

    fn tick_stepper(
        &mut self,
        now: std::time::Instant,
        out: &mut Vec<Action>,
        bindings: &[(&StickSide, &StepperParams)],
        mode: StepperMode,
    ) {
        let mut regs: Vec<RepeatReg> = Vec::new();
        for (side, params) in bindings.iter() {
            for (cid, axes_arr) in self.axes.iter() {
                let cid = *cid;
                let axes = *axes_arr;
                let (x, y) = (axes[Self::axis_index(CtrlAxis::LeftX)], axes[Self::axis_index(CtrlAxis::LeftY)]);
                let (rx, ry) = (axes[Self::axis_index(CtrlAxis::RightX)], axes[Self::axis_index(CtrlAxis::RightY)]);
                let (vx, vy) = match side { StickSide::Left => (x, y), StickSide::Right => (rx, ry) };
                let v = match params.axis { ProfileAxis::X => vx, ProfileAxis::Y => vy };
                let mag = v.abs();
                if mag < params.deadzone { continue; }
                let t = mag;
                let interval_ms = (params.max_interval_ms as f32) + (1.0 - t) * ((params.min_interval_ms as f32) - (params.max_interval_ms as f32));
                let positive = v >= 0.0;
                let key = mode.key_for(positive);
                let kind = mode.kind_for(params.axis, positive);
                let task_id = RepeatTaskId { controller: cid, side: **side, kind };
                regs.push(RepeatReg { id: task_id, key, fire_on_activate: true, initial_delay_ms: 0, interval_ms: interval_ms as u64 });
            }
        }
        for reg in regs.into_iter() {
            if let Some(a) = self.repeater_register(reg, now) { out.push(a); }
        }
    }

    fn repeater_register(
        &mut self,
        reg: RepeatReg,
        now: std::time::Instant,
    ) -> Option<Action> {
        match self.repeat_tasks.get_mut(&reg.id) {
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
                self.repeat_tasks.insert(reg.id, st);
                if reg.fire_on_activate { Some(Action::KeyTap(Arc::new(KeyCombo::from_key(reg.key)))) } else { None }
            }
        }
    }

    fn repeater_drain_due(&mut self, now: std::time::Instant, out: &mut Vec<Action>) {
        for (_id, st) in self.repeat_tasks.iter_mut() {
            let due_ms = if st.delay_done { st.interval_ms } else { st.initial_delay_ms };
            if due_ms == 0 { continue; }
            let elapsed = now.duration_since(st.last_fire).as_millis() as u64;
            if elapsed >= due_ms {
                out.push(Action::KeyTap(Arc::new(KeyCombo::from_key(st.key))));
                st.last_fire = now;
                st.delay_done = true;
            }
        }
    }

    fn repeater_cleanup_inactive(&mut self) {
        let gen = self.generation;
        self.repeat_tasks.retain(|_, st| st.last_seen_generation == gen);
    }

    fn tick_mouse(&mut self, out: &mut Vec<Action>, bindings: &[(&StickSide, &MouseParams)]) {
        for (side, params) in bindings.iter() {
            for (_cid, axes_arr) in self.axes.iter() {
                let axes = *axes_arr;
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (x, y) = Self::invert_xy(x0, y0, params.invert_x, params.invert_y);
                let mag_raw = Self::magnitude2d(x, y);
                if mag_raw < params.deadzone { continue; }
                let base = Self::normalize_after_deadzone(mag_raw, params.deadzone);
                let gamma = params.gamma.max(0.1);
                let mag = if (gamma - 1.0).abs() < 1e-6 { base } else if (gamma - 2.0).abs() < 1e-6 { base * base } else { base.powf(gamma) };
                if mag <= 0.0 { continue; }
                let dir_x = x / mag_raw;
                let dir_y = y / mag_raw;
                let speed_px_s = params.max_speed_px_s * mag;
                let dt_s = 0.010;
                let dx = (speed_px_s * dir_x * dt_s).round() as i32;
                let dy = (speed_px_s * dir_y * dt_s).round() as i32;
                if dx != 0 || dy != 0 { out.push(Action::MouseMove { dx, dy }); }
            }
        }
    }

    fn tick_scroll(&mut self, out: &mut Vec<Action>, bindings: &[(&StickSide, &ScrollParams)]) {
        for (side, params) in bindings.iter() {
            for (cid, axes_arr) in self.axes.iter() {
                let cid = *cid;
                let axes = *axes_arr;
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (mut x, y) = Self::invert_xy(x0, y0, params.invert_x, !params.invert_y);
                if !params.horizontal { x = 0.0; }
                let mag_raw = x.abs().max(y.abs());
                if mag_raw <= params.deadzone { continue; }
                let dt_s = 0.1;
                let accum = self.scroll_accum.entry((cid, **side)).or_insert((0.0_f32, 0.0_f32));
                accum.0 += params.speed_lines_s * x * dt_s;
                accum.1 += params.speed_lines_s * y * dt_s;
                let h = accum.0.round() as i32;
                let v = accum.1.round() as i32;
                if h != 0 { out.push(Action::Scroll { h, v: 0 }); accum.0 -= h as f32; }
                if v != 0 { out.push(Action::Scroll { h: 0, v }); accum.1 -= v as f32; }
            }
        }
    }
}
