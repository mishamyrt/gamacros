use gamacros_bit_derive::Bit;
use gamacros_bit_mask::{AtomicBitmask, Bitmask};

/// Realistic example: Macro pad with programmable buttons
#[derive(Bit, Debug, Clone, Copy, PartialEq, Eq)]
enum MacroButton {
    Button1,
    Button2,
    Button3,
    Button4,
    Button5,
    ModifierCtrl,
    ModifierAlt,
    ModifierShift,
}

/// Represents a macro action
#[derive(Debug, Clone)]
struct MacroAction {
    name: String,
    button_combo: Bitmask<MacroButton>,
}

impl MacroAction {
    fn new(name: &str, buttons: &[MacroButton]) -> Self {
        Self {
            name: name.to_string(),
            button_combo: Bitmask::new(buttons),
        }
    }

    fn matches(&self, pressed_buttons: &Bitmask<MacroButton>) -> bool {
        // Check if all required buttons are pressed
        self.button_combo.is_subset(pressed_buttons)
    }
}

fn main() {
    // Define some macro actions
    let macros = vec![
        MacroAction::new(
            "Copy",
            &[MacroButton::ModifierCtrl, MacroButton::Button1],
        ),
        MacroAction::new(
            "Paste",
            &[MacroButton::ModifierCtrl, MacroButton::Button2],
        ),
        MacroAction::new(
            "Select All",
            &[MacroButton::ModifierCtrl, MacroButton::Button3],
        ),
        MacroAction::new(
            "Save",
            &[MacroButton::ModifierCtrl, MacroButton::Button4],
        ),
        MacroAction::new(
            "New Tab",
            &[MacroButton::ModifierCtrl, MacroButton::ModifierShift, MacroButton::Button5],
        ),
    ];

    // Simulate button presses using atomic bitmask (thread-safe)
    let pressed_buttons = AtomicBitmask::empty();

    println!("=== Macro Pad Simulator ===\n");

    // Simulate various button combinations
    let scenarios = vec![
        ("Single button", vec![MacroButton::Button1]),
        ("Ctrl+C", vec![MacroButton::ModifierCtrl, MacroButton::Button1]),
        ("Ctrl+V", vec![MacroButton::ModifierCtrl, MacroButton::Button2]),
        ("Ctrl+A", vec![MacroButton::ModifierCtrl, MacroButton::Button3]),
        ("Ctrl+S", vec![MacroButton::ModifierCtrl, MacroButton::Button4]),
        ("Ctrl+Shift+T", vec![MacroButton::ModifierCtrl, MacroButton::ModifierShift, MacroButton::Button5]),
        ("Random combo", vec![MacroButton::Button1, MacroButton::Button3, MacroButton::ModifierAlt]),
    ];

    for (scenario_name, button_combo) in scenarios {
        println!("Scenario: {scenario_name}");

        // Press buttons
        for &button in &button_combo {
            pressed_buttons.insert(button);
        }

        // Get current state
        let current_state = pressed_buttons.load();
        println!("  Pressed buttons: {current_state:?}");

        // Check for matching macros
        let mut triggered_macros = vec![];
        for macro_action in &macros {
            if macro_action.matches(&current_state) {
                triggered_macros.push(macro_action.name.clone());
            }
        }

        if !triggered_macros.is_empty() {
            println!("  Triggered macros: {}", triggered_macros.join(", "));
        } else {
            println!("  No macros triggered");
        }

        // Clear buttons for next scenario
        for &button in &button_combo {
            pressed_buttons.remove(button);
        }
        println!();
    }

    // Demonstrate subset checking
    println!("=== Subset Analysis ===");
    let ctrl_combo = Bitmask::new(&[MacroButton::ModifierCtrl, MacroButton::Button1]);
    let full_combo = Bitmask::new(&[MacroButton::ModifierCtrl, MacroButton::Button1, MacroButton::ModifierAlt]);

    println!("Ctrl+C combo: {ctrl_combo:?}");
    println!("Full combo: {full_combo:?}");
    println!("Ctrl+C is subset of full: {}", ctrl_combo.is_subset(&full_combo));
    println!("Full is subset of Ctrl+C: {}", full_combo.is_subset(&ctrl_combo));

    // Show bit representations
    println!("\n=== Bit Representations ===");
    for button in [
        MacroButton::Button1, MacroButton::ModifierCtrl, MacroButton::Button5, MacroButton::ModifierShift
    ] {
        println!("{:?}: bit={:#b}, index={}", button, button.bit(), button.index());
    }
}
