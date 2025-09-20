pub mod gamacros;
pub mod stick;

pub use gamacros::{Gamacros, Action};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonPhase {
    Pressed,
    Released,
}
