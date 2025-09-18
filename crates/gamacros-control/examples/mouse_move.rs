use gamacros_control::Performer;

fn parse_i32(value: Option<String>, name: &str) -> i32 {
    match value {
        Some(v) => match v.parse::<i32>() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("Invalid {name} value '{v}'. Must be an integer.");
                std::process::exit(64);
            }
        },
        None => {
            eprintln!("Usage: mouse_move <x> <y>\nExample: mouse_move 120 -60");
            std::process::exit(64);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let x = parse_i32(args.next(), "x");
    let y = parse_i32(args.next(), "y");

    let mut performer = match Performer::new() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Failed to initialize input performer: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = performer.mouse_move(x, y) {
        eprintln!("Failed to move mouse by ({x}, {y}): {err}");
        std::process::exit(1);
    }
}
