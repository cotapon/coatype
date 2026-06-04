#![allow(clippy::upper_case_acronyms, dead_code)]
use crate::rdev::{EventType, Key, KeyboardState};
use core_graphics::event::CGEventFlags;

type ModifierState = u32;

pub struct Keyboard {
    dead_state: u32,
    shift: bool,
    caps_lock: bool,
}
impl Keyboard {
    pub fn new() -> Option<Keyboard> {
        Some(Keyboard {
            dead_state: 0,
            shift: false,
            caps_lock: false,
        })
    }

    fn modifier_state(&self) -> ModifierState {
        if self.caps_lock || self.shift {
            2
        } else {
            0
        }
    }

    pub(crate) unsafe fn create_string_for_key(
        &mut self,
        _code: u32,
        _flags: CGEventFlags,
    ) -> Option<String> {
        // Skip TIS/TSM key-name lookup: TISGetInputSourceProperty calls into
        // TSMGetInputSourceProperty which asserts main-queue on macOS 26+,
        // crashing the CGEventTap thread. We never use event.name in CoAType.
        None
    }

    pub(crate) unsafe fn string_from_code(
        &mut self,
        _code: u32,
        _modifier_state: ModifierState,
    ) -> Option<String> {
        None
    }
}

impl KeyboardState for Keyboard {
    fn add(&mut self, event_type: &EventType) -> Option<String> {
        match event_type {
            EventType::KeyPress(key) => match key {
                Key::ShiftLeft | Key::ShiftRight => {
                    self.shift = true;
                    None
                }
                Key::CapsLock => {
                    self.caps_lock = !self.caps_lock;
                    None
                }
                _key => None,
            },
            EventType::KeyRelease(key) => match key {
                Key::ShiftLeft | Key::ShiftRight => {
                    self.shift = false;
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.dead_state = 0;
        self.shift = false;
        self.caps_lock = false;
    }
}

#[allow(dead_code)]
pub unsafe fn flags_to_state(_flags: u64) -> ModifierState {
    0
}
