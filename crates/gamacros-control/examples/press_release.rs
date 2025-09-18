use gamacros_control::{KeyCombo, Performer};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let combo_str = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("Usage: press_release <combo> [delay_ms]\nExample: press_release cmd+a 200");
            std::process::exit(64);
        }
    };

    let delay_ms: u64 = args
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(200);

    let key_combo = match KeyCombo::from_str(&combo_str) {
        Ok(kc) => kc,
        Err(err) => {
            eprintln!("Failed to parse key combo '{combo_str}': {err}");
            std::process::exit(2);
        }
    };

    let mut performer = match Performer::new() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Failed to initialize input performer: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = performer.press(&key_combo) {
        eprintln!("Failed to press combo '{combo_str}': {err}");
        std::process::exit(1);
    }

    sleep(Duration::from_millis(delay_ms));

    if let Err(err) = performer.release(&key_combo) {
        eprintln!("Failed to release combo '{combo_str}': {err}");
        std::process::exit(1);
    }
}
