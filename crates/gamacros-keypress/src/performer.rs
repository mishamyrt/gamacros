use enigo::{Axis, Coordinate, Enigo, InputResult, Mouse, NewConError, Settings};

use crate::KeyCombo;

pub struct Performer {
    enigo: Enigo,
}

// SAFETY: This is safe because we're only accessing Enigo through a Mutex,
// which provides the necessary synchronization. The internal CGEventSource
// is only used on the thread that actually performs the key presses.
unsafe impl Send for Performer {}
unsafe impl Sync for Performer {}

impl Performer {
    pub fn new() -> Result<Self, NewConError> {
        let settings = Settings::default();
        let enigo = Enigo::new(&settings)?;
        Ok(Self { enigo })
    }

    pub fn perform(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        key_combo.perform(&mut self.enigo)
    }

    pub fn press(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        key_combo.press(&mut self.enigo)
    }

    pub fn release(&mut self, key_combo: &KeyCombo) -> InputResult<()> {
        key_combo.release(&mut self.enigo)
    }

    pub fn mouse_move(&mut self, x: i32, y: i32) -> InputResult<()> {
        self.enigo.move_mouse(x, y, Coordinate::Rel)
    }

    pub fn scroll_x(&mut self, value: i32) -> InputResult<()> {
        self.enigo.scroll(value, Axis::Horizontal)
    }

    pub fn scroll_y(&mut self, value: i32) -> InputResult<()> {
        self.enigo.scroll(value, Axis::Vertical)
    }
}
