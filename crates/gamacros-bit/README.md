# gamacros-bit

A Rust library for efficient bit manipulation and masking operations, designed for use cases like keyboard macro programming, sensor state tracking, and other scenarios requiring compact bit-level data structures.

## Modules

### `gamacros-bit-derive`
Procedural macro crate providing the `#[derive(Bit)]` macro for automatic implementation of the `Bitable` trait.

### `gamacros-bit-mask`
Core library providing `Bitmask` and `AtomicBitmask` types for bit-level operations.

## Quick Start

### Basic Usage with Derive Macro

```rust
use gamacros_bit_derive::Bit;
use gamacros_bit_mask::{Bitmask, Bitable};

#[derive(Bit, Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    A, B, X, Y, Start, Select
}

fn main() {
    // Create a button combination
    let combo = Bitmask::new(&[Button::X, Button::Y]);
    println!("Combo contains X: {}", combo.contains(Button::X));

    // Add a button
    let mut extended_combo = combo;
    extended_combo.insert(Button::Start);
    println!("Extended combo: {:?}", extended_combo);
}
```

### Manual Bitable Implementation

```rust
use gamacros_bit_mask::{Bitmask, Bitable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    Read, Write, Execute
}

impl Bitable for Permission {
    fn bit(&self) -> u64 {
        match self {
            Permission::Read => 1 << 0,
            Permission::Write => 1 << 1,
            Permission::Execute => 1 << 2,
        }
    }

    fn index(&self) -> u32 {
        match self {
            Permission::Read => 0,
            Permission::Write => 1,
            Permission::Execute => 2,
        }
    }
}
```

### Thread-Safe Operations

```rust
use std::sync::Arc;
use gamacros_bit_mask::AtomicBitmask;

let shared_mask = Arc::new(AtomicBitmask::empty());

// Use across multiple threads safely
shared_mask.insert(sensor_type);
```

## Examples

See the `examples/` directory in each module for comprehensive usage examples:

- `basic_usage.rs` - Fundamental bitmask operations
- `with_derive.rs` - Using the derive macro
- `atomic_usage.rs` - Thread-safe concurrent operations
- `macro_pad.rs` - Realistic macro pad implementation

Run examples with:
```bash
cargo run --example <example_name> --manifest-path crates/gamacros-bit/mask/Cargo.toml
```
