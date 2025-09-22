use gamacros_gamepad::ControllerId;
use gamacros_workspace::{Axis as ProfileAxis, StickMode, StickSide};

use crate::app::gamacros::Action;

use super::compiled::CompiledStickRules;
use super::repeat::{Direction, RepeatKind, RepeatTaskId, RepeatReg, StickProcessor};
use super::StepperMode;
use super::util::{
    axis_index, axes_for_side, invert_xy, magnitude2d, normalize_after_deadzone,
};

impl StickProcessor {
    pub fn on_tick_with<F: FnMut(Action)>(
        &mut self,
        bindings: Option<&CompiledStickRules>,
        axes_list: &[(ControllerId, [f32; 6])],
        mut sink: F,
    ) {
        if axes_list.is_empty() && !self.has_active_repeats() {
            return;
        }
        let Some(bindings) = bindings else {
            return;
        };

        let now = std::time::Instant::now();
        self.generation = self.generation.wrapping_add(1);

        if matches!(bindings.left(), Some(StickMode::Arrows(_)))
            || matches!(bindings.right(), Some(StickMode::Arrows(_)))
        {
            self.tick_arrows(now, &mut sink, axes_list, bindings);
        }
        if matches!(bindings.left(), Some(StickMode::Volume(_)))
            || matches!(bindings.right(), Some(StickMode::Volume(_)))
        {
            self.tick_stepper(
                now,
                &mut sink,
                axes_list,
                bindings,
                StepperMode::Volume,
            );
        }
        if matches!(bindings.left(), Some(StickMode::Brightness(_)))
            || matches!(bindings.right(), Some(StickMode::Brightness(_)))
        {
            self.tick_stepper(
                now,
                &mut sink,
                axes_list,
                bindings,
                StepperMode::Brightness,
            );
        }
        if matches!(bindings.left(), Some(StickMode::MouseMove(_)))
            || matches!(bindings.right(), Some(StickMode::MouseMove(_)))
        {
            self.tick_mouse(&mut sink, axes_list, bindings);
        }
        if matches!(bindings.left(), Some(StickMode::Scroll(_)))
            || matches!(bindings.right(), Some(StickMode::Scroll(_)))
        {
            self.tick_scroll(&mut sink, axes_list, bindings);
        }

        // Repeat draining is now event-driven, cleanup still needs to run per generation
        self.repeater_cleanup_inactive();
    }

    pub fn has_active_repeats(&self) -> bool {
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
        bindings: &CompiledStickRules,
    ) {
        let mut regs = std::mem::take(&mut self.regs);
        regs.clear();
        for (id, axes) in axes_list.iter().cloned() {
            if let Some(StickMode::Arrows(params)) = bindings.left() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Left);
                let (x, y) = invert_xy(x0, y0, params.invert_x, !params.invert_y);
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
                        side: StickSide::Left,
                        kind: RepeatKind::Arrow(dir),
                    };
                    let key = Self::get_direction_key(dir);
                    regs.push(RepeatReg {
                        id: task_id,
                        key,
                        fire_on_activate: true,
                        initial_delay_ms: params.repeat_delay_ms,
                        interval_ms: params.repeat_interval_ms,
                    });
                }
            }
            if let Some(StickMode::Arrows(params)) = bindings.right() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Right);
                let (x, y) = invert_xy(x0, y0, params.invert_x, !params.invert_y);
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
                        side: StickSide::Right,
                        kind: RepeatKind::Arrow(dir),
                    };
                    let key = Self::get_direction_key(dir);
                    regs.push(RepeatReg {
                        id: task_id,
                        key,
                        fire_on_activate: true,
                        initial_delay_ms: params.repeat_delay_ms,
                        interval_ms: params.repeat_interval_ms,
                    });
                }
            }
        }
        for reg in regs.drain(..) {
            if let Some(a) = self.repeater_register(reg, now) {
                (sink)(a);
            }
        }
        self.regs = regs;
    }

    fn tick_stepper(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &CompiledStickRules,
        mode: StepperMode,
    ) {
        let mut regs = std::mem::take(&mut self.regs);
        regs.clear();
        for (cid, axes) in axes_list.iter().cloned() {
            if let Some(step_params) = match (&mode, bindings.left()) {
                (StepperMode::Volume, Some(StickMode::Volume(p))) => Some(p),
                (StepperMode::Brightness, Some(StickMode::Brightness(p))) => Some(p),
                _ => None,
            } {
                let (vx, vy) = (
                    axes[axis_index(gamacros_gamepad::Axis::LeftX)],
                    axes[axis_index(gamacros_gamepad::Axis::LeftY)],
                );
                let v = match step_params.axis {
                    ProfileAxis::X => vx,
                    ProfileAxis::Y => vy,
                };
                let mag = v.abs();
                if mag >= step_params.deadzone {
                    let t = mag;
                    let interval_ms = (step_params.max_interval_ms as f32)
                        + (1.0 - t)
                            * ((step_params.min_interval_ms as f32)
                                - (step_params.max_interval_ms as f32));
                    let positive = v >= 0.0;
                    let key = mode.key_for(positive);
                    let kind = mode.kind_for(step_params.axis, positive);
                    let task_id = RepeatTaskId {
                        controller: cid,
                        side: StickSide::Left,
                        kind,
                    };
                    regs.push(RepeatReg {
                        id: task_id,
                        key,
                        fire_on_activate: true,
                        initial_delay_ms: 0,
                        interval_ms: interval_ms as u64,
                    });
                }
            }
            if let Some(step_params) = match (&mode, bindings.right()) {
                (StepperMode::Volume, Some(StickMode::Volume(p))) => Some(p),
                (StepperMode::Brightness, Some(StickMode::Brightness(p))) => Some(p),
                _ => None,
            } {
                let (vx, vy) = (
                    axes[axis_index(gamacros_gamepad::Axis::RightX)],
                    axes[axis_index(gamacros_gamepad::Axis::RightY)],
                );
                let v = match step_params.axis {
                    ProfileAxis::X => vx,
                    ProfileAxis::Y => vy,
                };
                let mag = v.abs();
                if mag >= step_params.deadzone {
                    let t = mag;
                    let interval_ms = (step_params.max_interval_ms as f32)
                        + (1.0 - t)
                            * ((step_params.min_interval_ms as f32)
                                - (step_params.max_interval_ms as f32));
                    let positive = v >= 0.0;
                    let key = mode.key_for(positive);
                    let kind = mode.kind_for(step_params.axis, positive);
                    let task_id = RepeatTaskId {
                        controller: cid,
                        side: StickSide::Right,
                        kind,
                    };
                    regs.push(RepeatReg {
                        id: task_id,
                        key,
                        fire_on_activate: true,
                        initial_delay_ms: 0,
                        interval_ms: interval_ms as u64,
                    });
                }
            }
        }
        for reg in regs.drain(..) {
            if let Some(a) = self.repeater_register(reg, now) {
                (sink)(a);
            }
        }
        self.regs = regs;
    }

    fn tick_mouse(
        &mut self,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &CompiledStickRules,
    ) {
        for (_cid, axes) in axes_list.iter().cloned() {
            if let Some(StickMode::MouseMove(params)) = bindings.left() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Left);
                let (x, y) = invert_xy(x0, y0, params.invert_x, params.invert_y);
                let mag_raw = magnitude2d(x, y);
                if mag_raw >= params.deadzone {
                    let base = normalize_after_deadzone(mag_raw, params.deadzone);
                    let mag = Self::fast_gamma(base, params.gamma);
                    if mag > 0.0 {
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
            if let Some(StickMode::MouseMove(params)) = bindings.right() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Right);
                let (x, y) = invert_xy(x0, y0, params.invert_x, params.invert_y);
                let mag_raw = magnitude2d(x, y);
                if mag_raw >= params.deadzone {
                    let base = normalize_after_deadzone(mag_raw, params.deadzone);
                    let mag = Self::fast_gamma(base, params.gamma);
                    if mag > 0.0 {
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
        }
    }

    #[inline]
    fn fast_gamma(base: f32, gamma: f32) -> f32 {
        let g = gamma.max(0.1);
        if (g - 1.0).abs() < 1e-6 {
            base
        } else if (g - 0.5).abs() < 1e-6 {
            base.sqrt()
        } else if (g - 1.5).abs() < 1e-6 {
            base * base.sqrt()
        } else if (g - 2.0).abs() < 1e-6 {
            base * base
        } else if (g - 3.0).abs() < 1e-6 {
            base * base * base
        } else {
            base.powf(g)
        }
    }

    fn tick_scroll(
        &mut self,
        sink: &mut impl FnMut(Action),
        axes_list: &[(ControllerId, [f32; 6])],
        bindings: &CompiledStickRules,
    ) {
        for (cid, axes) in axes_list.iter().cloned() {
            if let Some(StickMode::Scroll(params)) = bindings.left() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Left);
                let (mut x, y) =
                    invert_xy(x0, y0, params.invert_x, !params.invert_y);
                if !params.horizontal {
                    x = 0.0;
                }
                let mag_raw = x.abs().max(y.abs());
                if mag_raw > params.deadzone {
                    let dt_s = 0.1;
                    let sidx = super::util::side_index(&StickSide::Left);
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
            if let Some(StickMode::Scroll(params)) = bindings.right() {
                let (x0, y0) = axes_for_side(axes, &StickSide::Right);
                let (mut x, y) =
                    invert_xy(x0, y0, params.invert_x, !params.invert_y);
                if !params.horizontal {
                    x = 0.0;
                }
                let mag_raw = x.abs().max(y.abs());
                if mag_raw > params.deadzone {
                    let dt_s = 0.1;
                    let sidx = super::util::side_index(&StickSide::Right);
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

    #[inline]
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

    #[inline]
    pub fn get_direction_key(dir: Direction) -> gamacros_control::Key {
        match dir {
            Direction::Up => gamacros_control::Key::UpArrow,
            Direction::Down => gamacros_control::Key::DownArrow,
            Direction::Left => gamacros_control::Key::LeftArrow,
            Direction::Right => gamacros_control::Key::RightArrow,
        }
    }
}
