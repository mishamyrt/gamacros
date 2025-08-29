use gamacros_controller::Axis;
use gamacros_keypress::KeyCombo;
use gamacros_profile::StickSide;

pub(crate) struct StickEngine {
    // Per-controller axis state (normalized [-1,1])
    pub axes: dashmap::DashMap<u32, [f32; 6]>, // LeftX, LeftY, RightX, RightY, LT, RT
    pub last_step: dashmap::DashMap<(u32, StickSide, Axis), std::time::Instant>,
    pub arrows_pressed: dashmap::DashMap<(u32, StickSide), Option<Direction>>,
    pub arrows_last: dashmap::DashMap<(u32, StickSide), std::time::Instant>,
    pub arrows_delay_done: dashmap::DashMap<(u32, StickSide), bool>,
    pub scroll_accum: dashmap::DashMap<(u32, StickSide), (f32, f32)>,
}

impl StickEngine {
    pub fn new() -> Self {
        Self {
            axes: dashmap::DashMap::new(),
            last_step: dashmap::DashMap::new(),
            arrows_pressed: dashmap::DashMap::new(),
            arrows_last: dashmap::DashMap::new(),
            arrows_delay_done: dashmap::DashMap::new(),
            scroll_accum: dashmap::DashMap::new(),
        }
    }

    pub fn axis_index(axis: Axis) -> usize {
        match axis {
            Axis::LeftX => 0,
            Axis::LeftY => 1,
            Axis::RightX => 2,
            Axis::RightY => 3,
            Axis::LeftTrigger => 4,
            Axis::RightTrigger => 5,
        }
    }

    pub fn update_axis(&mut self, id: u32, axis: Axis, value: f32) {
        let idx = Self::axis_index(axis);
        let mut entry = self.axes.entry(id).or_insert([0.0; 6]);
        entry[idx] = value;
    }

    pub fn release_all_for(&self, id: u32) {
        for item in self.arrows_pressed.iter() {
            let (cid, _side) = *item.key();
            if cid != id { continue; }
        }
        self.arrows_pressed.retain(|(cid, _), _| *cid != id);
        self.last_step.retain(|(cid, _, _), _| *cid != id);
        self.arrows_last.retain(|(cid, _), _| *cid != id);
        self.arrows_delay_done.retain(|(cid, _), _| *cid != id);
        self.axes.remove(&id);
    }

    pub fn release_all_arrows(&self) {
        self.arrows_pressed.clear();
        self.arrows_last.clear();
        self.arrows_delay_done.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { Up, Down, Left, Right }
