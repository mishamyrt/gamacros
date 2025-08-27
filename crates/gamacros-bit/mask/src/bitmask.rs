use std::marker::PhantomData;

use crate::Bitable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitmask<T: Bitable>(pub u64, PhantomData<T>);

impl<T: Bitable> Bitmask<T> {
    /// Create a new bitmask from a slice of values.
    pub fn new(values: &[T]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < values.len() {
            bits |= values[i].bit();
            i += 1;
        }
        Self(bits, PhantomData)
    }

    /// Create an empty bitmask.
    pub const fn empty() -> Self {
        Self(0, PhantomData)
    }

    /// Create a new bitmask from a value.
    pub const fn from_value(value: u64) -> Self {
        Self(value, PhantomData)
    }

    /// Check if the bitmask contains a specific value.
    #[inline]
    pub fn contains(&self, bit: T) -> bool {
        (self.0 & bit.bit()) != 0
    }

    /// Insert a value to the bitmask.
    #[inline]
    pub fn insert(&mut self, bit: T) {
        self.0 |= bit.bit();
    }

    /// Remove a value from the bitmask.
    #[inline]
    pub fn remove(&mut self, bit: T) {
        self.0 &= !bit.bit();
    }

    /// Check if the bitmask is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Check if the bitmask is subset of another bitmask.
    #[inline]
    pub fn is_subset(&self, other: &Bitmask<T>) -> bool {
        self.0 & other.0 == self.0
    }

    /// Check if the bitmask is subset of another bitmask.
    #[inline]
    pub fn is_superset(&self, other: &Bitmask<T>) -> bool {
        other.is_subset(self)
    }

    /// Count the number of bits set in the bitmask.
    #[inline]
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::Bitmask;
    use crate::Bitable;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestFlag {
        A = 0,
        B = 1,
        C = 2,
        D = 3,
    }

    impl Bitable for TestFlag {
        fn bit(&self) -> u64 {
            1u64 << (*self as u64)
        }

        fn index(&self) -> u32 {
            *self as u32
        }
    }

    #[test]
    fn empty_creates_no_bits_set() {
        let mask = Bitmask::<TestFlag>::empty();
        assert!(!mask.contains(TestFlag::A));
        assert!(!mask.contains(TestFlag::B));
        assert!(!mask.contains(TestFlag::C));
        assert!(!mask.contains(TestFlag::D));
    }

    #[test]
    fn new_sets_bits_from_slice() {
        let mask = Bitmask::new(&[TestFlag::A, TestFlag::C]);
        assert!(mask.contains(TestFlag::A));
        assert!(!mask.contains(TestFlag::B));
        assert!(mask.contains(TestFlag::C));
        assert!(!mask.contains(TestFlag::D));
    }

    #[test]
    fn new_handles_duplicates() {
        let mask = Bitmask::new(&[TestFlag::B, TestFlag::B, TestFlag::D]);
        assert!(!mask.contains(TestFlag::A));
        assert!(mask.contains(TestFlag::B));
        assert!(!mask.contains(TestFlag::C));
        assert!(mask.contains(TestFlag::D));
    }

    #[test]
    fn insert_and_remove_toggle_bits() {
        let mut mask = Bitmask::empty();

        mask.insert(TestFlag::A);
        assert!(mask.contains(TestFlag::A));
        assert!(!mask.contains(TestFlag::B));

        mask.insert(TestFlag::B);
        assert!(mask.contains(TestFlag::A));
        assert!(mask.contains(TestFlag::B));

        mask.remove(TestFlag::A);
        assert!(!mask.contains(TestFlag::A));
        assert!(mask.contains(TestFlag::B));
    }

    #[test]
    fn is_empty_works() {
        let mut mask = Bitmask::empty();
        assert!(mask.is_empty());
        mask.insert(TestFlag::A);
        assert!(!mask.is_empty());
        mask.remove(TestFlag::A);
        assert!(mask.is_empty());
    }

    #[test]
    fn is_subset_works() {
        let empty = Bitmask::<TestFlag>::empty();
        let a = Bitmask::new(&[TestFlag::A]);
        let b = Bitmask::new(&[TestFlag::B]);
        let ab = Bitmask::new(&[TestFlag::A, TestFlag::B]);

        // empty is subset of any set
        assert!(empty.is_subset(&empty));
        assert!(empty.is_subset(&a));
        assert!(empty.is_subset(&ab));

        // self subset
        assert!(a.is_subset(&a));
        assert!(ab.is_subset(&ab));

        // proper subset
        assert!(a.is_subset(&ab));

        // not subset cases
        assert!(!ab.is_subset(&a));
        assert!(!a.is_subset(&b));
    }
}
