use crate::{Modifier, Modifiers};
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, InputResult, Key, Keyboard,
};
use serde::{
    de::{value::Error as DeError, IntoDeserializer},
    Deserializer,
};
use serde::{de::Visitor, Deserialize};
use std::fmt;

pub const ILLUMINATION_UP: Key = Key::IlluminationUp;
pub const ILLUMINATION_DOWN: Key = Key::IlluminationDown;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub keys: Vec<Key>,
}

impl KeyCombo {
    pub fn from_key(key: Key) -> Self {
        Self {
            modifiers: Modifiers::empty(),
            keys: vec![key],
        }
    }
}

fn parse_key(input: &str) -> Option<Key> {
    if input.is_empty() {
        return None;
    }

    if input.len() == 1 {
        let char = input.chars().next()?;
        return Some(Key::Unicode(char));
    }

    match input {
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

        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "page_up" => Some(Key::PageUp),
        "page_down" => Some(Key::PageDown),
        "delete" => Some(Key::Delete),
        "backspace" => Some(Key::Backspace),
        "escape" => Some(Key::Escape),
        "esc" => Some(Key::Escape),
        "arrow_up" => Some(Key::UpArrow),
        "arrow_down" => Some(Key::DownArrow),
        "arrow_left" => Some(Key::LeftArrow),
        "arrow_right" => Some(Key::RightArrow),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "spacebar" => Some(Key::Space),

        "enter" => Some(Key::Return),
        "return" => Some(Key::Return),

        "volume_up" => Some(Key::VolumeUp),
        "volume_down" => Some(Key::VolumeDown),
        "volume_mute" => Some(Key::VolumeMute),
        "brightness_up" => Some(Key::BrightnessUp),
        "brightness_down" => Some(Key::BrightnessDown),

        "illumination_up" => Some(Key::IlluminationUp),
        "illumination_down" => Some(Key::IlluminationDown),

        "'" | "quote" | "apostrophe" => Some(Key::Other(0x27)),
        ";" | "semicolon" => Some(Key::Other(0x29)),
        "\\" | "backslash" => Some(Key::Other(0x2A)),
        "`" | "grave" | "backtick" | "tilde" => Some(Key::Other(0x32)),
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

impl<'de> Deserialize<'de> for KeyCombo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyComboVisitor;

        impl Visitor<'_> for KeyComboVisitor {
            type Value = KeyCombo;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("key combination string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut modifiers: Modifiers = Modifiers::empty();
                let mut keys: Vec<Key> = Vec::new();
                for combo in v.split('+') {
                    let part = combo.trim();
                    match parse_key(part) {
                        Some(k) => match k {
                            Key::Control | Key::Meta | Key::Shift | Key::Alt => {
                                modifiers.add(Modifier::from(k));
                            }
                            _ => {
                                keys.push(k);
                            }
                        },
                        None => {
                            return Err(E::custom(format!("Invalid key: {part}")));
                        }
                    }
                }

                Ok(KeyCombo { modifiers, keys })
            }
        }

        deserializer.deserialize_str(KeyComboVisitor)
    }
}

impl KeyCombo {
    pub fn perform(&self, enigo: &mut Enigo) -> InputResult<()> {
        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control, Press)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta, Press)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift, Press)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt, Press)?;
        }

        for key in self.keys.clone() {
            enigo.key(key, Click)?;
        }

        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control, Release)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta, Release)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift, Release)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt, Release)?;
        }

        Ok(())
    }

    pub fn press(&self, enigo: &mut Enigo) -> InputResult<()> {
        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control, Press)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta, Press)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift, Press)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt, Press)?;
        }
        for key in self.keys.iter() {
            enigo.key(*key, Press)?;
        }

        Ok(())
    }

    pub fn release(&self, enigo: &mut Enigo) -> InputResult<()> {
        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control, Release)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta, Release)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift, Release)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt, Release)?;
        }
        for key in self.keys.iter() {
            enigo.key(*key, Release)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for KeyCombo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        KeyCombo::deserialize(s.into_deserializer())
            .map_err(|e: DeError| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::value::Error as DeError;
    use serde::de::IntoDeserializer;

    fn parse(input: &str) -> Result<KeyCombo, String> {
        KeyCombo::deserialize(input.into_deserializer())
            .map_err(|e: DeError| e.to_string())
    }

    #[test]
    fn test_single_modifier() {
        let kc = parse("ctrl").unwrap();
        assert!(kc.modifiers.contains(Modifier::Ctrl));
        assert!(kc.keys.is_empty());
    }

    #[test]
    fn test_multiple_modifiers() {
        let kc = parse("ctrl+alt+shift").unwrap();
        assert!(kc.modifiers.contains(Modifier::Ctrl));
        assert!(kc.modifiers.contains(Modifier::Alt));
        assert!(kc.modifiers.contains(Modifier::Shift));
        assert!(kc.keys.is_empty());
    }

    #[test]
    fn test_synonyms() {
        let kc = parse("cmd+option").unwrap();
        assert!(kc.modifiers.contains(Modifier::Meta));
        assert!(kc.modifiers.contains(Modifier::Alt));
        assert!(kc.keys.is_empty());
    }

    #[test]
    fn test_invalid_modifier() {
        let err = parse("ctrl+foo").unwrap_err();
        assert!(err.contains("Invalid key: foo"));
    }

    #[test]
    fn test_empty_string() {
        let Err(_) = parse("") else {
            panic!("Expected error");
        };
    }

    #[test]
    fn test_key_combo() {
        let kc = parse("ctrl+alt+shift+a").unwrap();
        assert!(kc.modifiers.contains(Modifier::Ctrl));
        assert!(kc.modifiers.contains(Modifier::Alt));
        assert!(kc.modifiers.contains(Modifier::Shift));
        assert_eq!(kc.keys.len(), 1);
        assert_eq!(kc.keys[0], Key::Unicode('a'));
    }
}
