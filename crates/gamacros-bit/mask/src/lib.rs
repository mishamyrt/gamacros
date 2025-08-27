mod bitmask;
mod atomic;

pub use bitmask::Bitmask;
pub use atomic::AtomicBitmask;

pub trait Bitable {
    fn bit(&self) -> u64;
    fn index(&self) -> u32;
}
