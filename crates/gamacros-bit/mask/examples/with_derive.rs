use gamacros_bit_derive::Bit;
use gamacros_bit_mask::{Bitmask};

/// Example using the derive macro to automatically implement Bitable
#[derive(Bit, Debug, Clone, Copy, PartialEq, Eq)]
enum GameButton {
    A,
    B,
    X,
    Y,
    Start,
    Select,
}

fn main() {
    // Create button combination for a fighting game move
    let combo = Bitmask::new(&[GameButton::X, GameButton::Y]);
    println!("Combo buttons: {combo:?}");

    // Check which buttons are pressed
    println!("X pressed: {}", combo.contains(GameButton::X));
    println!("B pressed: {}", combo.contains(GameButton::B));

    // Add start button to pause the game
    let mut paused = combo;
    paused.insert(GameButton::Start);
    println!("Paused state: {paused:?}");

    // Check if combo is subset of full controller state
    let full_controller = Bitmask::new(&[
        GameButton::A, GameButton::B, GameButton::X, GameButton::Y,
        GameButton::Start, GameButton::Select
    ]);
    println!("Combo is subset of full controller: {}", paused.is_subset(&full_controller));

    // Demonstrate bit values
    println!("Button bit values:");
    for button in [GameButton::A, GameButton::B, GameButton::X, GameButton::Y] {
        println!("  {:?}: bit={}, index={}", button, button.bit(), button.index());
    }
}
