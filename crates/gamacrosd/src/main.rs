mod gamacros;
mod logging;

use std::{time::Duration, fs};

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use fern::{Dispatch};

use gamacros_controller::{Button, ControllerEvent, ControllerManager};
use gamacros_keypress::Performer;
use gamacros_profile::{parse_profile, Action, Rule, Profile, TriggerPhase};
use gamacros_activity::{Monitor, Event as ActivityEvent, request_stop};

use crate::{gamacros::Gamacros};

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
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            handle_button(&gamacros, &mut keypress, &manager, id, button, TriggerPhase::Pressed);
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            handle_button(&gamacros, &mut keypress, &manager, id, button, TriggerPhase::Released);
                        }
                        Err(err) => {
                            print_error!("event channel closed: {err}");
                            break;
                        }
                    }
                }
            }
        }
    });

    // Start monitoring on the main thread (blocks until error/exit)
    let _ = monitor.start_listening();
    let _ = event_loop.join();
}

fn handle_button(
    gamacros: &Gamacros,
    keypress: &mut Performer,
    manager: &ControllerManager,
    id: u32,
    button: Button,
    phase: TriggerPhase,
) {
    let actions = gamacros.handle_button(id, button, phase);
    for action in actions {
        dispatch_action(gamacros, keypress, manager, id, action, phase);
    }
}

fn dispatch_action(
    gamacros: &Gamacros,
    keypress: &mut Performer,
    manager: &ControllerManager,
    id: u32,
    rule: Rule,
    phase: TriggerPhase,
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

    match rule.action {
        Action::Key(combo) => {
            // vibrate before action
            match rule.when {
                TriggerPhase::Pressed => match phase {
                    TriggerPhase::Pressed => {
                        vibrate(manager, id, rule.vibrate, supports_rumble);
                        let _ = keypress.press(&combo);
                    }
                    TriggerPhase::Released => {
                        let _ = keypress.release(&combo);
                    }
                },
                TriggerPhase::Released => {
                    vibrate(manager, id, rule.vibrate, supports_rumble);
                    let _ = keypress.perform(&combo);
                }
            }
        }
        Action::None => {}
    }
}
