use std::collections::HashMap;

use enigo::Key;
use gamacros_controller::{Axis as CtrlAxis, ControllerId};
use gamacros_keypress::KeyCombo;
use gamacros_profile::{ArrowsParams, Axis as ProfileAxis, MouseParams, ScrollParams, StepperParams, StickMode, StickRules, StickSide};

use crate::gamacros::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct StickProcessor {
    axes: HashMap<ControllerId, [f32; 6]>,
    last_step: HashMap<(ControllerId, StickSide, CtrlAxis), std::time::Instant>,
    arrows_pressed: HashMap<(ControllerId, StickSide), Option<Direction>>, // None for deadzone
    arrows_last: HashMap<(ControllerId, StickSide), std::time::Instant>,
    arrows_delay_done: HashMap<(ControllerId, StickSide), bool>,
    scroll_accum: HashMap<(ControllerId, StickSide), (f32, f32)>,
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
        self.arrows_pressed.retain(|(cid, _), _| *cid != id);
        self.last_step.retain(|(cid, _, _), _| *cid != id);
        self.arrows_last.retain(|(cid, _), _| *cid != id);
        self.arrows_delay_done.retain(|(cid, _), _| *cid != id);
        self.scroll_accum.retain(|(cid, _), _| *cid != id);
        self.axes.remove(&id);
    }

    pub fn release_all_arrows(&mut self) {
        self.arrows_pressed.clear();
        self.arrows_last.clear();
        self.arrows_delay_done.clear();
    }

    pub fn on_app_change(&mut self) {
        self.release_all_arrows();
        self.last_step.clear();
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

    pub fn get_direction_key(dir: Direction) -> KeyCombo {
        match dir {
            Direction::Up => KeyCombo::from_key(Key::UpArrow),
            Direction::Down => KeyCombo::from_key(Key::DownArrow),
            Direction::Left => KeyCombo::from_key(Key::LeftArrow),
            Direction::Right => KeyCombo::from_key(Key::RightArrow),
        }
    }

    pub fn on_tick(&mut self, bindings: &Option<StickRules>) -> Vec<Action> {
        let mut out: Vec<Action> = Vec::new();
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

        if !arrow_bindings.is_empty() {
            self.tick_arrows(&mut out, &arrow_bindings);
        }
        if !volume_bindings.is_empty() {
            self.tick_volume(&mut out, &volume_bindings);
        }
        if !brightness_bindings.is_empty() {
            self.tick_brightness(&mut out, &brightness_bindings);
        }
        if !mouse_bindings.is_empty() {
            self.tick_mouse(&mut out, &mouse_bindings);
        }
        if !scroll_bindings.is_empty() {
            self.tick_scroll(&mut out, &scroll_bindings);
        }

        out
    }

    fn tick_arrows(&mut self, out: &mut Vec<Action>, arrow_bindings: &[(&StickSide, &ArrowsParams)]) {
        for (id, axes_arr) in self.axes.iter() {
            let id = *id;
            let axes = *axes_arr;
            for (side, params) in arrow_bindings.iter() {
                let (x0, y0) = Self::axes_for_side(axes, side);
                let (x, y) = Self::invert_xy(x0, y0, params.invert_x, !params.invert_y);
                let mag = Self::magnitude2d(x, y);
                let new_dir = if mag < params.deadzone { None } else { Self::quantize_direction(x, y) };
                let key = (id, **side);
                let prev = self.arrows_pressed.get(&key).copied().unwrap_or(None);

                if prev != new_dir {
                    self.arrows_pressed.insert(key, new_dir);
                    if let Some(dir) = new_dir {
                        out.push(Action::KeyTap(Self::get_direction_key(dir)));
                        self.arrows_delay_done.insert(key, false);
                        self.arrows_last.insert(key, std::time::Instant::now());
                    } else {
                        self.arrows_delay_done.remove(&key);
                        self.arrows_last.remove(&key);
                    }
                    continue;
                }

                if let Some(dir) = new_dir {
                    let now = std::time::Instant::now();
                    let last = self.arrows_last.entry(key).or_insert(now);
                    let delay_done = self.arrows_delay_done.entry(key).or_insert(false);
                    let elapsed = now.duration_since(*last);
                    if !*delay_done {
                        if elapsed.as_millis() as u64 >= params.repeat_delay_ms {
                            out.push(Action::KeyTap(Self::get_direction_key(dir)));
                            *last = now;
                            *delay_done = true;
                        }
                    } else if elapsed.as_millis() as u64 >= params.repeat_interval_ms {
                        out.push(Action::KeyTap(Self::get_direction_key(dir)));
                        *last = now;
                    }
                }
            }
        }
    }

    fn tick_volume(&mut self, out: &mut Vec<Action>, bindings: &[(&StickSide, &StepperParams)]) {
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
                let key = if v >= 0.0 { KeyCombo::from_key(Key::VolumeUp) } else { KeyCombo::from_key(Key::VolumeDown) };
                let now = std::time::Instant::now();
                let c_axis = match (*side, params.axis) {
                    (StickSide::Left,  ProfileAxis::X) => CtrlAxis::LeftX,
                    (StickSide::Left,  ProfileAxis::Y) => CtrlAxis::LeftY,
                    (StickSide::Right, ProfileAxis::X) => CtrlAxis::RightX,
                    (StickSide::Right, ProfileAxis::Y) => CtrlAxis::RightY,
                };
                let last = self.last_step.entry((cid, **side, c_axis)).or_insert(now - std::time::Duration::from_millis(1000));
                let elapsed = now.duration_since(*last);
                if elapsed.as_millis() as u64 >= interval_ms as u64 { out.push(Action::KeyTap(key)); *last = now; }
            }
        }
    }

    fn tick_brightness(&mut self, out: &mut Vec<Action>, bindings: &[(&StickSide, &StepperParams)]) {
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
                let key = if v >= 0.0 { KeyCombo::from_key(Key::BrightnessUp) } else { KeyCombo::from_key(Key::BrightnessDown) };
                let now = std::time::Instant::now();
                let c_axis = match (*side, params.axis) {
                    (StickSide::Left,  ProfileAxis::X) => CtrlAxis::LeftX,
                    (StickSide::Left,  ProfileAxis::Y) => CtrlAxis::LeftY,
                    (StickSide::Right, ProfileAxis::X) => CtrlAxis::RightX,
                    (StickSide::Right, ProfileAxis::Y) => CtrlAxis::RightY,
                };
                let last = self.last_step.entry((cid, **side, c_axis)).or_insert(now - std::time::Duration::from_millis(1000));
                let elapsed = now.duration_since(*last);
                if elapsed.as_millis() as u64 >= interval_ms as u64 { out.push(Action::KeyTap(key)); *last = now; }
            }
        }
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
                if Self::normalize_after_deadzone(mag_raw, params.deadzone) <= 0.0 { continue; }
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
