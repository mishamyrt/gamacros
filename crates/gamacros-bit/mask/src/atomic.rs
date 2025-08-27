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