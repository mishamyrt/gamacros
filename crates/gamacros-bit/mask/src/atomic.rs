use std::{marker::PhantomData, sync::atomic::{AtomicU64, Ordering}};

use crate::{Bitable};
use crate::bitmask::Bitmask;

#[derive(Debug)]
pub struct AtomicBitmask<T: Bitable>(AtomicU64, PhantomData<T>);

impl<T: Bitable> AtomicBitmask<T> {
    /// Create a new atomic bitmask.
    pub fn new(values: &[T]) -> Self {
        let mask = Bitmask::new(values);
        Self(AtomicU64::new(mask.0), PhantomData)
    }

    /// Create a new atomic bitmask from a value.
    pub fn from_value(initial_value: u64) -> Self {
        Self(AtomicU64::new(initial_value), PhantomData)
    }

    /// Create an empty atomic bitmask.
    pub fn empty() -> Self {
        Self(AtomicU64::new(0), PhantomData)
    }

    /// Check if the atomic bitmask is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.load(Ordering::Relaxed) == 0
    }

    /// Insert a bit into the atomic bitmask.
    #[inline]
    pub fn insert(&self, bit: T) {
        self.0.fetch_or(bit.bit(), Ordering::Relaxed);
    }

    /// Remove a bit from the atomic bitmask.
    #[inline]
    pub fn remove(&self, bit: T) {
        self.0.fetch_and(!bit.bit(), Ordering::Relaxed);
    }

    /// Check if the atomic bitmask contains a bit.
    #[inline]
    pub fn contains(&self, bit: T) -> bool {
        let value = Bitmask::from_value(self.0.load(Ordering::Relaxed));
        value.contains(bit)
    }

    /// Check if the atomic bitmask is subset of another bitmask.
    #[inline]
    pub fn is_subset(&self, other: &Bitmask<T>) -> bool {
        let value = Bitmask::from_value(self.0.load(Ordering::Relaxed));
        value.is_subset(other)
    }

    /// Check if the atomic bitmask is superset of another bitmask.
    #[inline]
    pub fn is_superset(&self, other: &Bitmask<T>) -> bool {
        let value = Bitmask::from_value(self.0.load(Ordering::Relaxed));
        value.is_superset(other)
    }

    /// Load current value as a Bitmask.
    #[inline]
    pub fn load(&self) -> Bitmask<T> {
        Bitmask::from_value(self.0.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::{AtomicBitmask, Bitmask};
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
        let mask = AtomicBitmask::<TestFlag>::empty();
        assert!(mask.is_empty());
        assert!(!mask.contains(TestFlag::A));
        assert!(!mask.contains(TestFlag::B));
        assert!(!mask.contains(TestFlag::C));
        assert!(!mask.contains(TestFlag::D));
    }

    #[test]
    fn new_sets_bits_from_slice() {
        let mask = AtomicBitmask::new(&[TestFlag::A, TestFlag::C]);
        assert!(mask.contains(TestFlag::A));
        assert!(!mask.contains(TestFlag::B));
        assert!(mask.contains(TestFlag::C));
        assert!(!mask.contains(TestFlag::D));
    }

    #[test]
    fn from_value_creates_from_u64() {
        let mask = AtomicBitmask::<TestFlag>::from_value(5); // 101 in binary
        assert!(mask.contains(TestFlag::A)); // bit 0
        assert!(!mask.contains(TestFlag::B)); // bit 1
        assert!(mask.contains(TestFlag::C)); // bit 2
    }

    #[test]
    fn insert_and_remove_toggle_bits() {
        let mask = AtomicBitmask::empty();

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
        let mask = AtomicBitmask::empty();
        assert!(mask.is_empty());

        mask.insert(TestFlag::A);
        assert!(!mask.is_empty());

        mask.remove(TestFlag::A);
        assert!(mask.is_empty());
    }

    #[test]
    fn contains_works() {
        let mask = AtomicBitmask::new(&[TestFlag::B, TestFlag::D]);

        assert!(!mask.contains(TestFlag::A));
        assert!(mask.contains(TestFlag::B));
        assert!(!mask.contains(TestFlag::C));
        assert!(mask.contains(TestFlag::D));
    }

    #[test]
    fn is_subset_works() {
        let empty = AtomicBitmask::<TestFlag>::empty();
        let a = AtomicBitmask::new(&[TestFlag::A]);
        let _b = AtomicBitmask::new(&[TestFlag::B]);
        let ab = AtomicBitmask::new(&[TestFlag::A, TestFlag::B]);

        let empty_bitmask = Bitmask::empty();
        let a_bitmask = Bitmask::new(&[TestFlag::A]);
        let ab_bitmask = Bitmask::new(&[TestFlag::A, TestFlag::B]);

        // empty is subset of any set
        assert!(empty.is_subset(&empty_bitmask));
        assert!(empty.is_subset(&a_bitmask));
        assert!(empty.is_subset(&ab_bitmask));

        // self subset
        assert!(a.is_subset(&a_bitmask));
        assert!(ab.is_subset(&ab_bitmask));

        // proper subset
        assert!(a.is_subset(&ab_bitmask));

        // not subset cases
        assert!(!ab.is_subset(&a_bitmask));
        assert!(!a.is_subset(&Bitmask::new(&[TestFlag::B])));
    }

    #[test]
    fn is_superset_works() {
        let empty = AtomicBitmask::<TestFlag>::empty();
        let a = AtomicBitmask::new(&[TestFlag::A]);
        let ab = AtomicBitmask::new(&[TestFlag::A, TestFlag::B]);

        let empty_bitmask = Bitmask::empty();
        let a_bitmask = Bitmask::new(&[TestFlag::A]);
        let ab_bitmask = Bitmask::new(&[TestFlag::A, TestFlag::B]);

        // any set is superset of empty
        assert!(empty.is_superset(&empty_bitmask));
        assert!(a.is_superset(&empty_bitmask));
        assert!(ab.is_superset(&empty_bitmask));

        // self superset
        assert!(a.is_superset(&a_bitmask));
        assert!(ab.is_superset(&ab_bitmask));

        // proper superset
        assert!(ab.is_superset(&a_bitmask));

        // not superset cases
        assert!(!a.is_superset(&ab_bitmask));
        assert!(!a.is_superset(&Bitmask::new(&[TestFlag::B])));
    }

    #[test]
    fn load_returns_current_value() {
        let mask = AtomicBitmask::new(&[TestFlag::A, TestFlag::C]);
        let loaded = mask.load();

        assert!(loaded.contains(TestFlag::A));
        assert!(!loaded.contains(TestFlag::B));
        assert!(loaded.contains(TestFlag::C));
        assert!(!loaded.contains(TestFlag::D));
    }

    #[test]
    fn operations_persist_after_load() {
        let mask = AtomicBitmask::empty();

        mask.insert(TestFlag::B);
        let loaded1 = mask.load();
        assert!(loaded1.contains(TestFlag::B));

        mask.insert(TestFlag::D);
        let loaded2 = mask.load();
        assert!(loaded2.contains(TestFlag::B));
        assert!(loaded2.contains(TestFlag::D));

        mask.remove(TestFlag::B);
        let loaded3 = mask.load();
        assert!(!loaded3.contains(TestFlag::B));
        assert!(loaded3.contains(TestFlag::D));
    }
}