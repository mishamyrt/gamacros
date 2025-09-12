mod gamacros;
mod stick;

pub(crate) use gamacros::{Gamacros, Action};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonPhase {
    Pressed,
    Released,
}
