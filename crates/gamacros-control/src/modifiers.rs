use crate::key::Key;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Modifier {
    Ctrl,
    Meta,
    Shift,
    Alt,
}

impl Modifier {
    pub const CTRL: u8 = 1 << 0;
    pub const META: u8 = 1 << 1;
    pub const SHIFT: u8 = 1 << 2;
    pub const ALT: u8 = 1 << 3;

    pub const fn to_bitmap(&self) -> u8 {
        match self {
            Modifier::Ctrl => Self::CTRL,
            Modifier::Meta => Self::META,
            Modifier::Shift => Self::SHIFT,
            Modifier::Alt => Self::ALT,
        }
    }
}

impl From<Key> for Modifier {
    // type Err = String;

    fn from(key: Key) -> Self {
        match key {
            Key::Control => Modifier::Ctrl,
            Key::Meta => Modifier::Meta,
            Key::Shift => Modifier::Shift,
            Key::Alt => Modifier::Alt,
            _ => panic!("Invalid modifier key"),
        }
    }
}
impl From<u8> for Modifier {
    fn from(value: u8) -> Self {
        match value {
            Self::CTRL => Modifier::Ctrl,
            Self::META => Modifier::Meta,
            Self::SHIFT => Modifier::Shift,
            Self::ALT => Modifier::Alt,
            _ => panic!("Invalid modifier bitmap"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_values(values: &[Modifier]) -> Self {
        let mut modifiers = Self::empty();
        let mut i = 0;
        loop {
            if i >= values.len() {
                break;
            }
            modifiers.add(values[i]);
            i += 1;
        }
        modifiers
    }

    pub const fn add(&mut self, modifier: Modifier) {
        self.0 |= modifier.to_bitmap();
    }

    pub const fn remove(&mut self, modifier: Modifier) {
        self.0 &= !modifier.to_bitmap();
    }

    pub const fn contains(&self, modifier: Modifier) -> bool {
        self.0 & modifier.to_bitmap() != 0
    }

    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_to_bitmap_and_from() {
        assert_eq!(Modifier::from(Modifier::Ctrl.to_bitmap()), Modifier::Ctrl);
        assert_eq!(Modifier::from(Modifier::Meta.to_bitmap()), Modifier::Meta);
        assert_eq!(Modifier::from(Modifier::Shift.to_bitmap()), Modifier::Shift);
        assert_eq!(Modifier::from(Modifier::Alt.to_bitmap()), Modifier::Alt);
    }

    #[test]
    fn test_modifiers_new_and_is_empty() {
        let mods = Modifiers::empty();
        assert!(mods.is_empty());
        assert_eq!(mods.len(), 0);
    }

    #[test]
    fn test_modifiers_add_and_contains() {
        let mut mods = Modifiers::empty();
        mods.add(Modifier::Ctrl);
        assert!(mods.contains(Modifier::Ctrl));
        assert!(!mods.contains(Modifier::Alt));
    }

    #[test]
    fn test_modifiers_remove() {
        let mut mods = Modifiers::empty();
        mods.add(Modifier::Ctrl);
        mods.add(Modifier::Alt);
        mods.remove(Modifier::Ctrl);
        assert!(!mods.contains(Modifier::Ctrl));
        assert!(mods.contains(Modifier::Alt));
    }

    #[test]
    fn test_modifiers_len() {
        let mut mods = Modifiers::empty();
        assert_eq!(mods.len(), 0);
        mods.add(Modifier::Ctrl);
        assert_eq!(mods.len(), 1);
        mods.add(Modifier::Alt);
        assert_eq!(mods.len(), 2);
        mods.remove(Modifier::Ctrl);
        assert_eq!(mods.len(), 1);
    }

    #[test]
    fn test_modifiers_from_values() {
        let mods = Modifiers::from_values(&[Modifier::Ctrl, Modifier::Alt]);
        assert!(mods.contains(Modifier::Ctrl));
        assert!(mods.contains(Modifier::Alt));
        assert!(!mods.contains(Modifier::Meta));
        assert_eq!(mods.len(), 2);
    }
}
