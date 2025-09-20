use enigo::Key as EnigoKey;

/// A key that can be emulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Unicode(char),
    Control,
    RControl,
    Meta,
    RCommand,
    Shift,
    RShift,
    Alt,
    RAlt,
    Home,
    End,
    PageUp,
    PageDown,
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
    Delete,
    Backspace,
    Escape,
    Tab,
    Space,
    Return,
    VolumeUp,
    VolumeDown,
    VolumeMute,
    BrightnessUp,
    BrightnessDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,

    Apostrophe,
    Semicolon,
    Backslash,
    Grave,

    Other(u32),
}

pub(crate) fn key_code_for_key_string(ch: char) -> u16 {
    match ch {
        'a' => 0,
        's' => 1,
        'd' => 2,
        'f' => 3,
        'h' => 4,
        'g' => 5,
        'z' => 6,
        'x' => 7,
        'c' => 8,
        'v' => 9,
        'b' => 11,
        'q' => 12,
        'w' => 13,
        'e' => 14,
        'r' => 15,
        'y' => 16,
        't' => 17,
        // '1' => 18,
        // '2' => 19,
        // '3' => 20,
        // '4' => 21,
        // '6' => 22,
        // '5' => 23,
        '=' => 24,
        // '9' => 25,
        // '7' => 26,
        '-' => 27,
        '8' => 28,
        '0' => 29,
        ']' => 30,
        'o' => 31,
        'u' => 32,
        '[' => 33,
        'i' => 34,
        'p' => 35,
        'l' => 37,
        'j' => 38,
        '\'' => 39,
        'k' => 40,
        ';' => 41,
        '\\' => 42,
        ',' => 43,
        '/' => 44,
        'n' => 45,
        'm' => 46,
        '.' => 47,
        '*' => 67,
        '+' => 69,
        '`' => 50,
        _ => {
            unreachable!()
            //eprintln!("keyString {} Not Found. Aborting...", key_string);
            //std::process::exit(1);
        }
    }
}

impl From<Key> for EnigoKey {
    fn from(key: Key) -> Self {
        key.to_enigo()
    }
}

impl From<&Key> for EnigoKey {
    fn from(key: &Key) -> Self {
        key.to_enigo()
    }
}

impl Key {
    pub fn to_enigo(&self) -> EnigoKey {
        match self {
            Key::Control => EnigoKey::Control,
            Key::RControl => EnigoKey::RControl,
            Key::Meta => EnigoKey::Meta,
            Key::RCommand => EnigoKey::RCommand,
            Key::Shift => EnigoKey::Shift,
            Key::RShift => EnigoKey::RShift,
            Key::Alt => EnigoKey::Alt,
            Key::RAlt => EnigoKey::Alt,
            Key::Home => EnigoKey::Home,
            Key::End => EnigoKey::End,
            Key::PageUp => EnigoKey::PageUp,
            Key::PageDown => EnigoKey::PageDown,
            Key::UpArrow => EnigoKey::UpArrow,
            Key::DownArrow => EnigoKey::DownArrow,
            Key::LeftArrow => EnigoKey::LeftArrow,
            Key::RightArrow => EnigoKey::RightArrow,
            Key::Delete => EnigoKey::Delete,
            Key::Backspace => EnigoKey::Backspace,
            Key::Escape => EnigoKey::Escape,
            Key::Tab => EnigoKey::Tab,
            Key::Space => EnigoKey::Space,
            Key::Return => EnigoKey::Return,
            Key::VolumeUp => EnigoKey::VolumeUp,
            Key::VolumeDown => EnigoKey::VolumeDown,
            Key::VolumeMute => EnigoKey::VolumeMute,
            Key::BrightnessUp => EnigoKey::BrightnessUp,
            Key::BrightnessDown => EnigoKey::BrightnessDown,
            Key::F1 => EnigoKey::F1,
            Key::F2 => EnigoKey::F2,
            Key::F3 => EnigoKey::F3,
            Key::F4 => EnigoKey::F4,
            Key::F5 => EnigoKey::F5,
            Key::F6 => EnigoKey::F6,
            Key::F7 => EnigoKey::F7,
            Key::F8 => EnigoKey::F8,
            Key::F9 => EnigoKey::F9,
            Key::F10 => EnigoKey::F10,
            Key::F11 => EnigoKey::F11,
            Key::F12 => EnigoKey::F12,
            Key::F13 => EnigoKey::F13,
            Key::F14 => EnigoKey::F14,
            Key::F15 => EnigoKey::F15,
            Key::F16 => EnigoKey::F16,
            Key::F17 => EnigoKey::F17,
            Key::F18 => EnigoKey::F18,
            Key::F19 => EnigoKey::F19,
            Key::F20 => EnigoKey::F20,
            Key::Unicode(ch) => EnigoKey::Other(key_code_for_key_string(*ch) as u32),
            Key::Apostrophe => EnigoKey::Other(key_code_for_key_string('\'') as u32),
            Key::Semicolon => EnigoKey::Other(key_code_for_key_string(';') as u32),
            Key::Backslash => EnigoKey::Other(key_code_for_key_string('\\') as u32),
            Key::Grave => EnigoKey::Other(key_code_for_key_string('`') as u32),
            Key::Other(code) => EnigoKey::Other(*code),
        }
    }
}

/// Parse a key string into a `Key` enum.
///
/// This function is used to parse a key string into a `Key` enum.
/// It is used to parse the key string from the command line.
///
/// # Example
///
/// ```
/// let key = parse_key("a");
/// assert_eq!(key, Some(Key::Unicode('a')));
/// ```
pub(crate) fn parse_key(input: &str) -> Option<Key> {
    if input.is_empty() {
        return None;
    }

    if input.len() == 1 {
        let ch = input.chars().next().expect("input must be not empty");
        if ch.is_ascii_lowercase() {
            return Some(Key::Other(key_code_for_key_string(ch) as u32));
        }
    }

    match input {
        // Modifiers
        "ctrl" => Some(Key::Control),
        "rctrl" => Some(Key::RControl),
        "meta" => Some(Key::Meta),
        "rmeta" => Some(Key::RCommand),
        "cmd" => Some(Key::Meta),
        "rcmd" => Some(Key::RCommand),
        "command" => Some(Key::Meta),
        "rcommand" => Some(Key::RCommand),
        "super" => Some(Key::Meta),
        "rsuper" => Some(Key::RCommand),
        "shift" => Some(Key::Shift),
        "alt" => Some(Key::Alt),
        "option" => Some(Key::Alt),

        // Navigation
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "page_up" => Some(Key::PageUp),
        "page_down" => Some(Key::PageDown),
        "arrow_up" => Some(Key::UpArrow),
        "arrow_down" => Some(Key::DownArrow),
        "arrow_left" => Some(Key::LeftArrow),
        "arrow_right" => Some(Key::RightArrow),

        // Actions
        "delete" => Some(Key::Delete),
        "backspace" => Some(Key::Backspace),
        "escape" | "esc" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "space" | "spacebar" => Some(Key::Space),
        "enter" | "return" => Some(Key::Return),

        // Media
        "volume_up" => Some(Key::VolumeUp),
        "volume_down" => Some(Key::VolumeDown),
        "volume_mute" => Some(Key::VolumeMute),
        "brightness_up" => Some(Key::BrightnessUp),
        "brightness_down" => Some(Key::BrightnessDown),

        // Special characters
        // Using codes from
        "'" | "quote" | "apostrophe" => Some(Key::Apostrophe),
        ";" | "semicolon" => Some(Key::Semicolon),
        "\\" | "backslash" => Some(Key::Backslash),
        "`" | "grave" | "backtick" | "tilde" => Some(Key::Grave),

        // Provide ANSI letter scancode aliases to avoid single-char Unicode path
        "ansi_k" => Some(Key::Other(0x28)),
        "ansi_n" => Some(Key::Other(0x2D)),
        "ansi_m" => Some(Key::Other(0x2E)),
        // Keypad (numpad) keys
        "kp_decimal" | "keypad_decimal" => Some(Key::Other(0x41)),
        "kp_multiply" | "keypad_multiply" => Some(Key::Other(0x43)),
        "kp_plus" | "keypad_plus" => Some(Key::Other(0x45)),
        "kp_clear" | "keypad_clear" => Some(Key::Other(0x47)),
        "kp_divide" | "keypad_divide" => Some(Key::Other(0x4B)),
        "kp_enter" | "keypad_enter" => Some(Key::Other(0x4C)),
        "kp_minus" | "keypad_minus" => Some(Key::Other(0x4E)),
        "kp_equals" | "keypad_equals" => Some(Key::Other(0x51)),
        "kp_0" | "keypad_0" => Some(Key::Other(0x52)),
        "kp_1" | "keypad_1" => Some(Key::Other(0x53)),
        "kp_2" | "keypad_2" => Some(Key::Other(0x54)),
        "kp_3" | "keypad_3" => Some(Key::Other(0x55)),
        "kp_4" | "keypad_4" => Some(Key::Other(0x56)),
        "kp_5" | "keypad_5" => Some(Key::Other(0x57)),
        "kp_6" | "keypad_6" => Some(Key::Other(0x58)),
        "kp_7" | "keypad_7" => Some(Key::Other(0x59)),
        "kp_8" | "keypad_8" => Some(Key::Other(0x5B)),
        "kp_9" | "keypad_9" => Some(Key::Other(0x5C)),
        "." | "period" | "dot" => Some(Key::Other(0x2f)),
        "," | "comma" => Some(Key::Other(0x2b)),
        "/" | "slash" => Some(Key::Other(0x2c)),
        "-" | "minus" => Some(Key::Other(0x1b)),
        "=" | "equal" => Some(Key::Other(0x18)),

        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "f13" => Some(Key::F13),
        "f14" => Some(Key::F14),
        "f15" => Some(Key::F15),
        "f16" => Some(Key::F16),
        "f17" => Some(Key::F17),
        "f18" => Some(Key::F18),
        "f19" => Some(Key::F19),
        "f20" => Some(Key::F20),
        _ => None,
    }
}
