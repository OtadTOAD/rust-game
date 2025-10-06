#![allow(dead_code)]

use std::collections::HashSet;

use winit::event::VirtualKeyCode;

pub struct InputState {
    keys_pressed: HashSet<VirtualKeyCode>,
    keys_just_pressed: HashSet<VirtualKeyCode>,
    keys_just_released: HashSet<VirtualKeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
    }

    pub fn press_key(&mut self, key: VirtualKeyCode) {
        if !self.keys_pressed.contains(&key) {
            self.keys_just_pressed.insert(key);
        }
        self.keys_pressed.insert(key);
    }

    pub fn release_key(&mut self, key: VirtualKeyCode) {
        if self.keys_pressed.contains(&key) {
            self.keys_just_released.insert(key);
        }
        self.keys_pressed.remove(&key);
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_key_just_released(&self, key: VirtualKeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }
}
