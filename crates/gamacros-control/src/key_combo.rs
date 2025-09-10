use crate::{key::{parse_key, Key}, Modifier, Modifiers};
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, InputResult, Keyboard,
};
use smallvec::SmallVec;
use serde::{
    de::{value::Error as DeError, IntoDeserializer},
    Deserializer,
};
use serde::{de::Visitor, Deserialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub keys: SmallVec<[Key; 4]>,
}

impl KeyCombo {
    pub fn from_key(key: Key) -> Self {
        Self {
            modifiers: Modifiers::empty(),
            keys: {
                let mut v: SmallVec<[Key; 4]> = SmallVec::new();
                v.push(key);
                v
            },
        }
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
                let mut keys: SmallVec<[Key; 4]> = SmallVec::new();
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
            enigo.key(Key::Control.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt.into(), Press)?;
        }

        for key in self.keys.iter() {
            enigo.key(key.into(), Click)?;
        }

        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt.into(), Release)?;
        }

        Ok(())
    }

    pub fn press(&self, enigo: &mut Enigo) -> InputResult<()> {
        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift.into(), Press)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt.into(), Press)?;
        }
        for key in self.keys.iter() {
            enigo.key(key.into(), Press)?;
        }

        Ok(())
    }

    pub fn release(&self, enigo: &mut Enigo) -> InputResult<()> {
        if self.modifiers.contains(Modifier::Ctrl) {
            enigo.key(Key::Control.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Meta) {
            enigo.key(Key::Meta.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Shift) {
            enigo.key(Key::Shift.into(), Release)?;
        }
        if self.modifiers.contains(Modifier::Alt) {
            enigo.key(Key::Alt.into(), Release)?;
        }
        for key in self.keys.iter() {
            enigo.key(key.into(), Release)?;
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
    use crate::key::key_code_for_key_string;

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
        assert_eq!(kc.keys[0], Key::Other(key_code_for_key_string('a') as u32));
    }
}
