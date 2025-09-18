use gamacros_control::{KeyCombo, Performer};
use std::str::FromStr;

fn main() {
    let mut args = std::env::args().skip(1);
    let combo_str = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("Usage: perform_key_combo <combo>\nExample: perform_key_combo ctrl+alt+shift+a");
            std::process::exit(64);
        }
    };

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

    if let Err(err) = performer.perform(&key_combo) {
        eprintln!("Failed to perform combo '{combo_str}': {err}");
        std::process::exit(1);
    }
}
