use gamacros_control::Performer;

fn parse_i32_opt(arg: Option<String>, name: &str) -> Option<i32> {
    arg.map(|v| match v.parse::<i32>() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("Invalid {name} value '{v}'. Must be an integer.");
            std::process::exit(64);
        }
    })
}

fn main() {
    // Usage: scroll <x> <y>
    let mut args = std::env::args().skip(1);
    let x = parse_i32_opt(args.next(), "x").unwrap_or(0);
    let y = parse_i32_opt(args.next(), "y").unwrap_or(0);

    if x == 0 && y == 0 {
        eprintln!(
            "Usage: scroll <x> <y>\nExample: scroll 0 -3  # scroll down 3 steps"
        );
        std::process::exit(64);
    }

    let mut performer = match Performer::new() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Failed to initialize input performer: {err}");
            std::process::exit(1);
        }
    };

    if x != 0 {
        if let Err(err) = performer.scroll_x(x) {
            eprintln!("Failed to scroll horizontally by {x}: {err}");
            std::process::exit(1);
        }
    }

    if y != 0 {
        if let Err(err) = performer.scroll_y(y) {
            eprintln!("Failed to scroll vertically by {y}: {err}");
            std::process::exit(1);
        }
    }
}
