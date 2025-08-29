mod gamacros;
mod logging;
mod stick;
// mod axis_bus;

use std::{time::Duration, fs};

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use enigo::Key;
use fern::{Dispatch};

use gamacros_controller::{ControllerEvent, ControllerManager, Axis};
use gamacros_keypress::{KeyCombo, Performer};
use gamacros_profile::{
    parse_profile, ArrowsParams, ButtonPhase, ButtonRule, MouseParams, Profile,
    ScrollParams, StepperParams, StickMode, StickSide, Axis as ProfileAxis,
};
use gamacros_activity::{Monitor, Event as ActivityEvent, request_stop};

use crate::{
    gamacros::Gamacros,
    stick::{Direction, StickEngine},
};

fn setup_logging(verbose: bool, no_color: bool) {
    let log_level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    Dispatch::new()
        .level(log_level)
        .chain(std::io::stdout())
        .apply()
        .expect("Unable to set up logger");

    if no_color {
        colored::control::set_override(false);
    }
}

fn load_profile() -> Profile {
    let profile_path = std::env::current_dir()
        .ok()
        .map(|p| p.join("profile.yaml"))
        .unwrap_or_else(|| std::path::PathBuf::from("profile.yaml"));
    let input = fs::read_to_string(&profile_path).ok().unwrap();
    parse_profile(&input).unwrap()
}

fn main() {
    setup_logging(false, false);

    // Handle Ctrl+C to exit cleanly
    let (stop_tx, stop_rx) = unbounded::<()>();
    ctrlc::set_handler(move || {
        let _ = stop_tx.send(());
        // also request the NSApplication run loop to stop
        request_stop();
    })
    .expect("failed to set Ctrl+C handler");

    // Activity monitor must run on the main thread.
    // We'll bridge its std::mpsc receiver to crossbeam.
    let (monitor, activity_std_rx) = Monitor::new();
    let (activity_tx, activity_rx) = crossbeam_channel::unbounded::<ActivityEvent>();
    std::thread::spawn(move || {
        while let Ok(ev) = activity_std_rx.recv() {
            let _ = activity_tx.send(ev);
        }
    });

    // Run the main event loop in a background thread while the main thread runs the monitor loop.
    let event_loop = std::thread::spawn(move || {
        let manager =
            ControllerManager::new().expect("failed to start controller manager");
        let rx = manager.subscribe();
        let mut keypress = Performer::new().expect("failed to start keypress");
        let mut stick_engine = StickEngine::new();
        let ticker = crossbeam_channel::tick(Duration::from_millis(10));

        // TODO: add file watch and hot-reload.
        let profile = load_profile();
        let gamacros = Gamacros::new(profile);
        print_info!(
            "gamacrosd started. Listening for controller and activity events."
        );
        loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(activity_rx) -> msg => {
                    match msg {
                        Ok(ActivityEvent::AppChange(bundle_id)) => {
                            if let Err(e) = gamacros.set_active_app(&bundle_id) {
                                print_error!("failed to set active app: {e}");
                                break;
                            }
                            // Invalidate stick state on app change to avoid stuck repeats
                            stick_engine.last_step.clear();
                            // Release any held arrows across all controllers on app change
                            stick_engine.release_all_arrows();
                        }
                        Ok(_) => {}
                        Err(_) => {
                            // Activity channel closed; stop the event loop to avoid spin.
                            break;
                        }
                    }
                }
                recv(rx) -> msg => {
                    match msg {
                        Ok(ControllerEvent::Connected(info)) => {
                            let id = info.id;
                            if gamacros.is_known(id) {
                                continue;
                            }

                            if let Err(e) = gamacros.add_controller(info) {
                                print_error!("failed to add controller: {e}");
                            }
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            if let Err(e) = gamacros.remove_controller(id) {
                                print_error!("failed to remove device: {e}");
                                break;
                            }
                            // Release internal state for this controller
                            stick_engine.release_all_for(id);
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            let phase = ButtonPhase::Pressed;
                            let actions = gamacros.handle_button(id, button, phase);
                            for action in actions {
                                dispatch_button_action(&gamacros, &mut keypress, &manager, id, action, phase);
                            }
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            let phase = ButtonPhase::Released;
                            let actions = gamacros.handle_button(id, button, phase);
                            for action in actions {
                                dispatch_button_action(&gamacros, &mut keypress, &manager, id, action, phase);
                            }
                        }
                        Ok(ControllerEvent::AxisMotion { id, axis, value }) => {
                            stick_engine.update_axis(id, axis, value);
                        }
                        Err(err) => {
                            print_error!("event channel closed: {err}");
                            break;
                        }
                    }
                }
                recv(ticker) -> _ => {
                    let Some(bindings) = gamacros.get_stick_bindings() else {
                        continue;
                    };

                    // Group bindings by mode to avoid duplicate processing
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


                    // Process arrows once per tick
                    if !arrow_bindings.is_empty() {
                        process_arrows(&mut keypress, &stick_engine, &arrow_bindings);
                    }

                    // Process volume per controller
                    for (side, params) in volume_bindings.iter() {
                        for entry in stick_engine.axes.iter() {
                            let cid = *entry.key();
                            process_stepper(&mut keypress, &stick_engine, cid, side, params, KeyCombo::from_key(Key::VolumeUp), KeyCombo::from_key(Key::VolumeDown));
                        }
                    }

                    // Process brightness per controller
                    for (side, params) in brightness_bindings.iter() {
                        for entry in stick_engine.axes.iter() {
                            let cid = *entry.key();
                            process_stepper(&mut keypress, &stick_engine, cid, side, params, KeyCombo::from_key(Key::BrightnessUp), KeyCombo::from_key(Key::BrightnessDown));
                        }
                    }

                    // Process mouse move
                    for binding in mouse_bindings.iter() {
                        process_mouse_move(&mut keypress, &stick_engine, binding);
                    }

                    // Process scroll with accumulator to avoid bounce
                    for binding in scroll_bindings.iter() {
                        process_scroll(&mut keypress, &stick_engine, binding);
                    }
                }
            }
        }
    });

    // Start monitoring on the main thread (blocks until error/exit)
    if let Err(e) = monitor.start_listening() {
        print_error!("activity monitor error: {e}");
        return;
    }
    if let Err(e) = event_loop.join() {
        print_error!("event loop error: {e:?}");
    }
}

// Small helpers to keep hot paths concise and consistent
fn axes_for_side(axes: [f32; 6], side: &StickSide) -> (f32, f32) {
    match side {
        StickSide::Left => (
            axes[StickEngine::axis_index(Axis::LeftX)],
            axes[StickEngine::axis_index(Axis::LeftY)],
        ),
        StickSide::Right => (
            axes[StickEngine::axis_index(Axis::RightX)],
            axes[StickEngine::axis_index(Axis::RightY)],
        ),
    }
}

fn invert_xy(x: f32, y: f32, invert_x: bool, invert_y: bool) -> (f32, f32) {
    let nx = if invert_x { -x } else { x };
    let ny = if invert_y { -y } else { y };
    (nx, ny)
}

fn magnitude2d(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

fn normalize_after_deadzone(mag: f32, deadzone: f32) -> f32 {
    if mag <= deadzone {
        0.0
    } else {
        ((mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0)
    }
}

fn process_mouse_move(
    keypress: &mut Performer,
    engine: &StickEngine,
    binding: &(&StickSide, &MouseParams),
) {
    let (side, params) = binding;
    for entry in engine.axes.iter() {
        let axes = *entry.value();
        let (x0, y0) = axes_for_side(axes, side);
        let (x, y) = invert_xy(x0, y0, params.invert_x, params.invert_y);
        let mag_raw = magnitude2d(x, y);
        if mag_raw < params.deadzone {
            continue;
        }
        // normalize after deadzone to avoid drift
        let base = normalize_after_deadzone(mag_raw, params.deadzone);
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
        let dt_s = 0.010; // 10ms ticker
        let dx = (speed_px_s * dir_x * dt_s).round() as i32;
        let dy = (speed_px_s * dir_y * dt_s).round() as i32;
        if dx != 0 || dy != 0 {
            let _ = keypress.mouse_move(dx, dy);
        }
    }
}

fn process_scroll(
    keypress: &mut Performer,
    engine: &StickEngine,
    binding: &(&StickSide, &ScrollParams),
) {
    let (side, params) = binding;
    for entry in engine.axes.iter() {
        let cid = *entry.key();
        let axes = *entry.value();
        let (x0, y0) = axes_for_side(axes, side);
        let (mut x, y) = invert_xy(x0, y0, params.invert_x, !params.invert_y); // natural mapping: up scrolls up
        if !params.horizontal {
            x = 0.0;
        }
        let mag_raw = x.abs().max(y.abs());
        if normalize_after_deadzone(mag_raw, params.deadzone) <= 0.0 {
            continue;
        }
        let dt_s = 0.1;
        let mut accum = engine
            .scroll_accum
            .entry((cid, **side))
            .or_insert((0.0_f32, 0.0_f32));
        accum.0 += params.speed_lines_s * x * dt_s;
        accum.1 += params.speed_lines_s * y * dt_s;
        let h = accum.0.round() as i32;
        let v = accum.1.round() as i32;
        if h != 0 {
            let _ = keypress.scroll_x(h);
            accum.0 -= h as f32;
        }
        if v != 0 {
            let _ = keypress.scroll_y(v);
            accum.1 -= v as f32;
        }
    }
}

fn process_arrows(
    keypress: &mut Performer,
    engine: &StickEngine,
    bindings: &[(&StickSide, &ArrowsParams)],
) {
    // For each controller with axes, process arrow bindings
    for entry in engine.axes.iter() {
        let id = *entry.key();
        let axes = *entry.value();
        for (side, params) in bindings.iter() {
            // select vector by stick
            let (x0, y0) = axes_for_side(axes, side);
            let (x, y) = invert_xy(x0, y0, params.invert_x, !params.invert_y); // default natural mapping up->Up; allow inversion via invert_y
            let mag = magnitude2d(x, y);
            let new_dir = if mag < params.deadzone {
                None
            } else {
                quantize_direction(x, y)
            };
            let key = (id, **side);
            let prev = engine.arrows_pressed.get(&key).and_then(|v| *v.value());

            if prev != new_dir {
                // direction changed
                engine.arrows_pressed.insert(key, new_dir);
                if let Some(dir) = new_dir {
                    // fire immediately for responsiveness
                    let _ = keypress.perform(&get_direction_key(dir));
                    engine.arrows_delay_done.insert(key, false);
                    engine.arrows_last.insert(key, std::time::Instant::now());
                } else {
                    // into deadzone: clear timers
                    engine.arrows_delay_done.remove(&key);
                    engine.arrows_last.remove(&key);
                }
                continue;
            }

            // Same direction held: handle repeat scheduling
            if let Some(dir) = new_dir {
                let now = std::time::Instant::now();
                let mut last = engine.arrows_last.entry(key).or_insert(now);
                let mut delay_done =
                    engine.arrows_delay_done.entry(key).or_insert(false);
                let elapsed = now.duration_since(*last);
                if !*delay_done {
                    if elapsed.as_millis() as u64 >= params.repeat_delay_ms {
                        let _ = keypress.perform(&get_direction_key(dir));
                        *last = now;
                        *delay_done = true;
                    }
                } else if elapsed.as_millis() as u64 >= params.repeat_interval_ms {
                    let _ = keypress.perform(&get_direction_key(dir));
                    *last = now;
                }
            }
        }
    }
}

fn get_direction_key(dir: Direction) -> KeyCombo {
    match dir {
        Direction::Up => KeyCombo::from_key(Key::UpArrow),
        Direction::Down => KeyCombo::from_key(Key::DownArrow),
        Direction::Left => KeyCombo::from_key(Key::LeftArrow),
        Direction::Right => KeyCombo::from_key(Key::RightArrow),
    }
}

fn process_stepper(
    keypress: &mut Performer,
    engine: &StickEngine,
    id: u32,
    side: &StickSide,
    params: &StepperParams,
    positive_key: KeyCombo,
    negative_key: KeyCombo,
) {
    let axes = match engine.axes.get(&id) {
        Some(v) => *v.value(),
        None => return,
    };
    let (x, y) = (
        axes[StickEngine::axis_index(Axis::LeftX)],
        axes[StickEngine::axis_index(Axis::LeftY)],
    );
    let (rx, ry) = (
        axes[StickEngine::axis_index(Axis::RightX)],
        axes[StickEngine::axis_index(Axis::RightY)],
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
        return;
    }
    // Map magnitude to interval [min, max] (larger deflection -> smaller interval)
    let t = mag; // linear; could be gamma later
    let interval_ms = (params.max_interval_ms as f32)
        + (1.0 - t)
            * ((params.min_interval_ms as f32) - (params.max_interval_ms as f32));
    let key = if v >= 0.0 {
        &positive_key
    } else {
        &negative_key
    };
    let now = std::time::Instant::now();
    let c_axis = match (*side, params.axis) {
        (StickSide::Left,  ProfileAxis::X) => Axis::LeftX,
        (StickSide::Left,  ProfileAxis::Y) => Axis::LeftY,
        (StickSide::Right, ProfileAxis::X) => Axis::RightX,
        (StickSide::Right, ProfileAxis::Y) => Axis::RightY,
    };
    let mut last = engine
        .last_step
        .entry((id, *side, c_axis))
        .or_insert(now - std::time::Duration::from_millis(1000));
    let elapsed = now.duration_since(*last);
    if elapsed.as_millis() as u64 >= interval_ms as u64 {
        let _ = keypress.perform(key);
        *last = now;
    }
}

fn quantize_direction(x: f32, y: f32) -> Option<Direction> {
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
    } else {
        // ax == ay, tie-breaker: prefer vertical
        if y > 0.0 {
            Some(Direction::Up)
        } else if y < 0.0 {
            Some(Direction::Down)
        } else {
            None
        }
    }
}

fn dispatch_button_action(
    gamacros: &Gamacros,
    keypress: &mut Performer,
    manager: &ControllerManager,
    id: u32,
    rule: ButtonRule,
    phase: ButtonPhase,
) {
    fn vibrate(
        manager: &ControllerManager,
        id: u32,
        vibrate: Option<u16>,
        supports_rumble: bool,
    ) {
        if !supports_rumble {
            return;
        }
        if let Some(ms) = vibrate {
            if let Some(h) = manager.controller(id) {
                let _ = h.rumble(0.2, 0.2, Duration::from_millis(ms as u64));
            }
        }
    }

    let supports_rumble = gamacros.supports_rumble(id);

    match rule.when {
        ButtonPhase::Pressed => match phase {
            ButtonPhase::Pressed => {
                vibrate(manager, id, rule.vibrate, supports_rumble);
                let _ = keypress.press(&rule.action);
            }
            ButtonPhase::Released => {
                let _ = keypress.release(&rule.action);
            }
        },
        ButtonPhase::Released => {
            vibrate(manager, id, rule.vibrate, supports_rumble);
            let _ = keypress.perform(&rule.action);
        }
    }
}
