mod gamacros;

use gamacros_controller::{Button, ControllerEvent, ControllerManager};
use gamacros_keypress::Performer;
use gamacros_profile::{parse_profile, Action, Rule, Profile, TriggerPhase};
use gamacros_activity::{Monitor, Event as ActivityEvent, request_stop};
use std::{time::Duration, fs};
use crossbeam_channel::{select, unbounded};

use crate::{gamacros::Gamacros};

fn load_profile() -> Profile {
    let profile_path = std::env::current_dir()
        .ok()
        .map(|p| p.join("profile.yaml"))
        .unwrap_or_else(|| std::path::PathBuf::from("profile.yaml"));
    let input = fs::read_to_string(&profile_path).ok().unwrap();
    parse_profile(&input).unwrap()
}

fn main() {
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

        for info in manager.controllers() {
            if let Err(e) =
                gamacros.add_device(info.id, info.vendor_id, info.product_id)
            {
                eprintln!("Failed to add device: {e}");
            }
        }

        println!("gamacrosd started. Listening for controller and activity events.");
        loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(activity_rx) -> msg => {
                    match msg {
                        Ok(ActivityEvent::AppChange(bundle_id)) => {
                            if let Err(e) = gamacros.set_active_app(&bundle_id) {
                                eprintln!("Failed to set active app: {e}");
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
                            if let Err(e) = gamacros.add_device(info.id, info.vendor_id, info.product_id) {
                                eprintln!("Failed to add device: {e}");
                            }
                            if let Some(h) = manager.controller(info.id) {
                                let _ = h.rumble(0.2, 0.2, Duration::from_millis(120));
                            }
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            if let Err(e) = gamacros.remove_device(id) {
                                eprintln!("Failed to remove device: {e}");
                            }
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            handle_button(&gamacros, &mut keypress, &manager, id, button, TriggerPhase::Pressed);
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            handle_button(&gamacros, &mut keypress, &manager, id, button, TriggerPhase::Released);
                        }
                        Err(err) => {
                            eprintln!("Event channel closed: {err}");
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
        dispatch_action(keypress, manager, id, action, phase);
    }
}

fn dispatch_action(
    keypress: &mut Performer,
    manager: &ControllerManager,
    id: u32,
    rule: Rule,
    phase: TriggerPhase,
) {
    fn vibrate(manager: &ControllerManager, id: u32, vibrate: Option<u16>) {
        if let Some(ms) = vibrate {
            if let Some(h) = manager.controller(id) {
                let _ = h.rumble(0.2, 0.2, Duration::from_millis(ms as u64));
            }
        }
    }

    match rule.action {
        Action::Key(combo) => {
            // vibrate before action
            match rule.when {
                TriggerPhase::Pressed => match phase {
                    TriggerPhase::Pressed => {
                        println!("pressed");
                        vibrate(manager, id, rule.vibrate);
                        let _ = keypress.press(&combo);
                    }
                    TriggerPhase::Released => {
                        println!("released");
                        let _ = keypress.release(&combo);
                    }
                },
                TriggerPhase::Released => {
                    println!("released");
                    vibrate(manager, id, rule.vibrate);
                    let _ = keypress.perform(&combo);
                }
            }
        }
        Action::None => {}
    }
}
