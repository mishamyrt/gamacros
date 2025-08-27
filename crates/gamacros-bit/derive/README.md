# gamacros-macros

Proc-macro helpers for gamacros.

## Derives

- `Bitmask`: derive on a fieldless enum to generate constant bit utilities.

### Generated API

- `const fn bit(self) -> u64` – returns a unique bit for the variant, based on declaration order.
- `const fn index(self) -> u32` – returns the bit index (trailing zeros of `bit()`).

### Example

```rust
use gamacros_macros::Bitmask;

#[derive(Bitmask, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Button { A, B, X, Y }

fn main() {
    assert_eq!(Button::A.bit(), 1u64 << 0);
    assert_eq!(Button::B.bit(), 1u64 << 1);
}
```


