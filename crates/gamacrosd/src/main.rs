mod runtime;

use gamacros_controller::{ControllerEvent, ControllerManager};
use gamacros_keypress::Performer;
use gamacros_profile::{Profile, TriggerPhase};
use gamacros_activity::{Monitor, Event as ActivityEvent, request_stop};
use std::{collections::HashMap, time::Duration, fs};
use crossbeam_channel::{select, unbounded};

use crate::runtime::{build_runtime, ProfileRuntime, RuntimeAction};

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
        let manager = ControllerManager::new().expect("failed to start controller manager");
        let rx = manager.subscribe();
        let mut keypress = Performer::new().expect("failed to start keypress");

        // TODO: add file watch and hot-reload.
        let profile_path = std::env::current_dir()
            .ok()
            .map(|p| p.join("profile.yaml"))
            .unwrap_or_else(|| std::path::PathBuf::from("profile.yaml"));
        let profile = fs::read_to_string(&profile_path)
            .ok()
            .and_then(|s| Profile::from_yaml_str(&s).ok())
            .unwrap_or_else(Profile::empty);

        let mut runtimes: HashMap<u32, ProfileRuntime> = HashMap::new();
        let mut current_app: String = String::new();

        // Seed runtimes for already-known controllers (enumerated before our subscription)
        for info in manager.controllers() {
            let runtime =
                build_runtime(&profile, info.vendor_id, info.product_id, &current_app);
            runtimes.insert(info.id, runtime);
        }

        println!("gamacrosd started. Listening for controller and activity events.");
        loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(activity_rx) -> msg => {
                    if let Ok(ActivityEvent::AppChange(bundle_id)) = msg {
                        println!("App change: {bundle_id}");
                        current_app = bundle_id.clone();
                        for rt in runtimes.values_mut() {
                            rt.set_app(&current_app);
                        }
                    }
                }
                recv(rx) -> msg => {
                    match msg {
                        Ok(ControllerEvent::Connected(info)) => {
                            let runtime = build_runtime(&profile, info.vendor_id, info.product_id, &current_app);
                            runtimes.insert(info.id, runtime);
                            if let Some(h) = manager.controller(info.id) {
                                let _ = h.rumble(0.2, 0.2, Duration::from_millis(120));
                            }
                        }
                        Ok(ControllerEvent::Disconnected(id)) => {
                            runtimes.remove(&id);
                        }
                        Ok(ControllerEvent::ButtonPressed { id, button }) => {
                            if let std::collections::hash_map::Entry::Vacant(e) = runtimes.entry(id) {
                                if let Some(info) = manager.controllers().into_iter().find(|i| i.id == id) {
                                    let runtime = build_runtime(&profile, info.vendor_id, info.product_id, &current_app);
                                e.insert(runtime);
                                }
                            }
                            if let Some(rt) = runtimes.get_mut(&id) {
                                for action in rt.on_button(button, TriggerPhase::Pressed) {
                                    println!("dispatching action: {action:?}");
                                    dispatch_action(&mut keypress, &manager, id, action);
                                }
                            }
                        }
                        Ok(ControllerEvent::ButtonReleased { id, button }) => {
                            if let std::collections::hash_map::Entry::Vacant(e) = runtimes.entry(id) {
                                if let Some(info) = manager.controllers().into_iter().find(|i| i.id == id) {
                                    let runtime = build_runtime(&profile, info.vendor_id, info.product_id, &current_app);
                                e.insert(runtime);
                                }
                            }
                            if let Some(rt) = runtimes.get_mut(&id) {
                                for action in rt.on_button(button, TriggerPhase::Released) {
                                    dispatch_action(&mut keypress, &manager, id, action);
                                }
                            }
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

fn dispatch_action(
    keypress: &mut Performer,
    manager: &ControllerManager,
    id: u32,
    action: RuntimeAction,
) {
    println!("dispatch_action: {action:?}");
    if let Some(kc) = action.key_combo {
        let _ = keypress.perform(&kc);
    }
    if let Some(ms) = action.vibrate_ms {
        if let Some(h) = manager.controller(id) {
            let _ = h.rumble(0.2, 0.2, Duration::from_millis(ms as u64));
        }
    }
}
