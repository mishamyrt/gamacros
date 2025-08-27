use std::sync::Arc;
use std::thread;
use std::time::Duration;

use gamacros_bit_mask::{AtomicBitmask, Bitable};

/// Example demonstrating thread-safe atomic bitmask operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sensor {
    Temperature,
    Motion,
}

impl Bitable for Sensor {
    fn bit(&self) -> u64 {
        match self {
            Sensor::Temperature => 1 << 0,
            Sensor::Motion => 1 << 3,
        }
    }

    fn index(&self) -> u32 {
        match self {
            Sensor::Temperature => 0,
            Sensor::Motion => 3,
        }
    }
}

fn main() {
    // Shared atomic bitmask for sensor states
    let sensor_states = Arc::new(AtomicBitmask::empty());

    // Spawn multiple threads to simulate sensor readings
    let mut handles = vec![];

    // Thread 1: Temperature sensor
    let states1 = Arc::clone(&sensor_states);
    let handle1 = thread::spawn(move || {
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(100));
            states1.insert(Sensor::Temperature);
            println!("Temperature sensor: reading taken");
            thread::sleep(Duration::from_millis(50));
            states1.remove(Sensor::Temperature);
            println!("Temperature sensor: reading processed");
        }
    });

    // Thread 2: Motion sensor
    let states2 = Arc::clone(&sensor_states);
    let handle2 = thread::spawn(move || {
        for _ in 0..3 {
            thread::sleep(Duration::from_millis(150));
            states2.insert(Sensor::Motion);
            println!("Motion sensor: movement detected");
            thread::sleep(Duration::from_millis(200));
            states2.remove(Sensor::Motion);
            println!("Motion sensor: movement processed");
        }
    });

    // Thread 3: Monitor thread
    let states3 = Arc::clone(&sensor_states);
    let handle3 = thread::spawn(move || {
        for i in 0..10 {
            thread::sleep(Duration::from_millis(50));
            let current_state = states3.load();
            println!("Monitor #{i}: Current active sensors: {current_state:?}");

            // Check for specific sensor states
            if current_state.contains(Sensor::Temperature) {
                println!("  -> Temperature sensor is active");
            }
            if current_state.contains(Sensor::Motion) {
                println!("  -> Motion sensor is active");
            }
        }
    });

    handles.push(handle1);
    handles.push(handle2);
    handles.push(handle3);

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Final state
    let final_state = sensor_states.load();
    println!("Final sensor state: {final_state:?}");
    println!("All sensors idle: {}", final_state.is_empty());
}
