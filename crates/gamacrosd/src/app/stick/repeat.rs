use ahash::AHashMap;
use gamacros_control::Key;
use gamacros_gamepad::ControllerId;
use gamacros_workspace::{Axis as ProfileAxis, StickSide};

use crate::app::gamacros::Action;

use super::util::{side_index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct StickProcessor {
    pub(super) controllers: AHashMap<ControllerId, ControllerRepeatState>,
    pub(super) generation: u64,
    pub(super) regs: Vec<RepeatReg>,
}

#[derive(Default)]
pub(super) struct ControllerRepeatState {
    pub(super) sides: [SideRepeatState; 2],
}

#[derive(Default)]
pub(super) struct SideRepeatState {
    pub(super) scroll_accum: (f32, f32),
    pub(super) arrows: [Option<RepeatTaskState>; 4],
    pub(super) volume: [Option<RepeatTaskState>; 4],
    pub(super) brightness: [Option<RepeatTaskState>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepeatKind {
    Arrow(Direction),
    Volume { axis: ProfileAxis, positive: bool },
    Brightness { axis: ProfileAxis, positive: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct RepeatTaskId {
    pub(super) controller: ControllerId,
    pub(super) side: StickSide,
    pub(super) kind: RepeatKind,
}

pub(super) struct RepeatTaskState {
    pub(super) key: Key,
    pub(super) fire_on_activate: bool,
    pub(super) initial_delay_ms: u64,
    pub(super) interval_ms: u64,
    pub(super) last_fire: std::time::Instant,
    pub(super) delay_done: bool,
    pub(super) last_seen_generation: u64,
}

pub(super) struct RepeatReg {
    pub(super) id: RepeatTaskId,
    pub(super) key: Key,
    pub(super) fire_on_activate: bool,
    pub(super) initial_delay_ms: u64,
    pub(super) interval_ms: u64,
}

impl StickProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn dir_index(dir: Direction) -> usize {
        match dir {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        }
    }

    pub(super) fn step_slot_index(axis: ProfileAxis, positive: bool) -> usize {
        match (axis, positive) {
            (ProfileAxis::X, false) => 0,
            (ProfileAxis::X, true) => 1,
            (ProfileAxis::Y, false) => 2,
            (ProfileAxis::Y, true) => 3,
        }
    }

    pub fn release_all_for(&mut self, id: ControllerId) {
        self.controllers.remove(&id);
    }

    pub fn release_all_arrows(&mut self) {
        for (_cid, state) in self.controllers.iter_mut() {
            for s in 0..2 {
                for slot in state.sides[s].arrows.iter_mut() {
                    *slot = None;
                }
            }
        }
    }

    pub fn on_app_change(&mut self) {
        self.release_all_arrows();
        for (_cid, state) in self.controllers.iter_mut() {
            for s in 0..2 {
                state.sides[s].scroll_accum = (0.0, 0.0);
            }
        }
    }

    pub(super) fn repeater_register(
        &mut self,
        reg: RepeatReg,
        now: std::time::Instant,
    ) -> Option<Action> {
        let cid = reg.id.controller;
        let side_idx = side_index(&reg.id.side);
        let ctrl = self.controllers.entry(cid).or_default();
        let side = &mut ctrl.sides[side_idx];

        let slot: &mut Option<RepeatTaskState> = match reg.id.kind {
            RepeatKind::Arrow(dir) => {
                let idx = Self::dir_index(dir);
                &mut side.arrows[idx]
            }
            RepeatKind::Volume { axis, positive } => {
                let idx = Self::step_slot_index(axis, positive);
                &mut side.volume[idx]
            }
            RepeatKind::Brightness { axis, positive } => {
                let idx = Self::step_slot_index(axis, positive);
                &mut side.brightness[idx]
            }
        };

        match slot {
            Some(st) => {
                st.key = reg.key;
                st.interval_ms = reg.interval_ms;
                st.initial_delay_ms = reg.initial_delay_ms;
                st.fire_on_activate = reg.fire_on_activate;
                st.last_seen_generation = self.generation;
                None
            }
            None => {
                let st = RepeatTaskState {
                    key: reg.key,
                    fire_on_activate: reg.fire_on_activate,
                    initial_delay_ms: reg.initial_delay_ms,
                    interval_ms: reg.interval_ms,
                    last_fire: now,
                    delay_done: reg.initial_delay_ms == 0,
                    last_seen_generation: self.generation,
                };
                *slot = Some(st);
                if reg.fire_on_activate {
                    Some(Action::KeyTap(gamacros_control::KeyCombo::from_key(
                        reg.key,
                    )))
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn repeater_drain_due(
        &mut self,
        now: std::time::Instant,
        sink: &mut impl FnMut(Action),
    ) {
        for (_cid, ctrl) in self.controllers.iter_mut() {
            for side in ctrl.sides.iter_mut() {
                for slot in side
                    .arrows
                    .iter_mut()
                    .chain(side.volume.iter_mut())
                    .chain(side.brightness.iter_mut())
                {
                    if let Some(st) = slot.as_mut() {
                        let due_ms = if st.delay_done {
                            st.interval_ms
                        } else {
                            st.initial_delay_ms
                        };
                        if due_ms == 0 {
                            continue;
                        }
                        let elapsed =
                            now.duration_since(st.last_fire).as_millis() as u64;
                        if elapsed >= due_ms {
                            (sink)(Action::KeyTap(
                                gamacros_control::KeyCombo::from_key(st.key),
                            ));
                            st.last_fire = now;
                            st.delay_done = true;
                        }
                    }
                }
            }
        }
    }

    pub(super) fn repeater_cleanup_inactive(&mut self) {
        let gen = self.generation;
        for (_cid, ctrl) in self.controllers.iter_mut() {
            for side in ctrl.sides.iter_mut() {
                for slot in side.arrows.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
                for slot in side.volume.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
                for slot in side.brightness.iter_mut() {
                    if let Some(st) = slot.as_ref() {
                        if st.last_seen_generation != gen {
                            *slot = None;
                        }
                    }
                }
            }
        }
    }
}
