mod gamacros;
mod logging;
mod stick;
// mod axis_bus;

use std::{time::Duration, fs};

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use fern::{Dispatch};

use gamacros_gamepad::{ControllerEvent, ControllerManager};
use gamacros_control::Performer;
use gamacros_profile::{parse_profile, ButtonPhase, Profile};
use gamacros_activity::{Monitor, Event as ActivityEvent, request_stop};

use crate::gamacros::{Gamacros, Action};

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
    setup_logging(true, false);

    // Handle Ctrl+C to exit cleanly
    let (stop_tx, stop_rx) = unbounded::<()>();
    ctrlc::set_handler(move || {
        let _ = stop_tx.send(());
        // also request the NSApplication run loop to stop
        request_stop();
    })
    .expect("failed to set Ctrl+C handler");

    // Activity monitor must run on the main thread.
    // We keep its std::mpsc receiver and poll it from the event loop (no bridge thread).
    let (monitor, activity_std_rx) = Monitor::new();

    // Run the main event loop in a background thread while the main thread runs the monitor loop.
    let event_loop = std::thread::Builder::new()
        .name("event-loop".into())
        .stack_size(512 * 1024)
        .spawn(move || {
        let manager =
            ControllerManager::new().expect("failed to start controller manager");
        let rx = manager.subscribe();
        let mut keypress = Performer::new().expect("failed to start keypress");
        // Stick processing is owned by Gamacros now
        let ticker = crossbeam_channel::tick(Duration::from_millis(10));

        // TODO: add file watch and hot-reload.
        let profile = load_profile();
        let mut gamacros = Gamacros::new(profile);
        print_info!(
            "gamacrosd started. Listening for controller and activity events."
        );
        loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(rx) -> msg => {
                    match msg {
                        Ok(ControllerEvent::Connected(info)) => {
                            let id = info.id;
                            if gamacros.is_known(id) {
                                continue;
                            }

                            gamacros.add_controller(info)
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            gamacros.remove_controller(id);
                            gamacros.on_controller_disconnected(id);
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            let actions = gamacros.on_button(id, button, ButtonPhase::Pressed);
                            for action in actions { apply_action(&mut keypress, &manager, action); }
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            let actions = gamacros.on_button(id, button, ButtonPhase::Released);
                            for action in actions { apply_action(&mut keypress, &manager, action); }
                        }
                        Ok(ControllerEvent::AxisMotion { id, axis, value }) => {
                            gamacros.on_axis_motion(id, axis, value);
                        }
                        Err(err) => {
                            print_error!("event channel closed: {err}");
                            break;
                        }
                    }
                }
                recv(ticker) -> _ => {
                    let actions = gamacros.on_tick();
                    for action in actions { apply_action(&mut keypress, &manager, action); }
                }
            }
            while let Ok(msg) = activity_std_rx.try_recv() {
                if let ActivityEvent::AppChange(bundle_id) = msg {
                    gamacros.set_active_app(&bundle_id)
                }
            }
        }
    }).expect("failed to spawn event loop thread");

    // Start monitoring on the main thread (blocks until error/exit)
    if let Err(e) = monitor.start_listening() {
        print_error!("activity monitor error: {e}");
        return;
    }
    if let Err(e) = event_loop.join() {
        print_error!("event loop error: {e:?}");
    }
}

// stick processing is handled inside Gamacros

fn apply_action(
    keypress: &mut Performer,
    manager: &ControllerManager,
    action: Action,
) {
    match action {
        Action::KeyTap(k) => {
            let _ = keypress.perform(&k);
        }
        Action::KeyPress(k) => {
            let _ = keypress.press(&k);
        }
        Action::KeyRelease(k) => {
            let _ = keypress.release(&k);
        }
        Action::MouseMove { dx, dy } => {
            let _ = keypress.mouse_move(dx, dy);
        }
        Action::Scroll { h, v } => {
            if h != 0 {
                let _ = keypress.scroll_x(h);
            }
            if v != 0 {
                let _ = keypress.scroll_y(v);
            }
        }
        Action::Rumble { id, ms } => {
            if let Some(h) = manager.controller(id) {
                let _ = h.rumble(0.2, 0.2, Duration::from_millis(ms as u64));
            }
        }
    }
}
