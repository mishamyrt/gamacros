use std::thread;

use gamacros_activity::Monitor;

fn main() {
    let (monitor, event_rx) = Monitor::new();

    thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            println!("Event: {:?}", event);
        }
    });

    monitor.start_listening().unwrap();
}
