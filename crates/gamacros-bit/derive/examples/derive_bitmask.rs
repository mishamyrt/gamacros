use gamacros_bit_derive::Bit;

#[derive(Bit, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Button {
    A,
    B,
    X,
    Y,
}

fn main() {
    assert_eq!(Button::A.bit(), 1u64 << 0);
    assert_eq!(Button::B.bit(), 1u64 << 1);
    assert_eq!(Button::X.bit(), 1u64 << 2);
    assert_eq!(Button::Y.bit(), 1u64 << 3);
}
