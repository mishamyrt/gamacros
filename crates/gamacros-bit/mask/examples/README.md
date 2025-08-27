# Examples for gamacros-bit-mask

This directory contains examples demonstrating various use cases for the gamacros-bit-mask library.

## Running Examples

To run an example, use the following command from the project root:

```bash
cargo run --example <example_name> --manifest-path crates/gamacros-bit/mask/Cargo.toml
```

For example:
```bash
cargo run --example basic_usage --manifest-path crates/gamacros-bit/mask/Cargo.toml
```

## Available Examples

### 1. `basic_usage.rs`
Demonstrates fundamental Bitmask operations:
- Creating bitmasks from values
- Checking for bit presence
- Inserting and removing bits
- Subset operations

### 2. `with_derive.rs`
Shows how to use the derive macro for automatic Bitable implementation:
- Using `#[derive(Bit)]` on enums
- Automatic bit value generation
- Working with derived types

### 3. `atomic_usage.rs`
Demonstrates thread-safe operations with AtomicBitmask:
- Concurrent sensor state tracking
- Thread-safe bit operations
- Monitoring shared state across threads

### 4. `macro_pad.rs`
Realistic example of a programmable macro pad:
- Complex button combinations
- Macro action matching
- Subset analysis for key combinations
- Practical use case demonstration

## Key Concepts Demonstrated

- **Bitable Trait**: How to implement custom types that can be used as bits
- **Bitmask Operations**: Creating, modifying, and querying bitmasks
- **Atomic Operations**: Thread-safe bitmask operations
- **Subset Logic**: Checking relationships between different bitmasks
- **Derive Macro**: Automatic implementation generation

## Dependencies

The examples use:
- `gamacros_bit_mask`: The main library
- `gamacros_bit_derive`: For the derive macro examples (where applicable)

For examples that use the derive macro, make sure both crates are available in your Cargo workspace.
