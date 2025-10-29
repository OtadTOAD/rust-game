use std::collections::HashSet;

use winit::event::{ElementState, VirtualKeyCode};

#[derive(Clone, Debug)]
pub enum InputEvent {
    KeyPressed(VirtualKeyCode),
    #[allow(dead_code)]
    KeyReleased(VirtualKeyCode),
    MouseMoved(f64, f64),
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}

impl Action {
    pub fn from_key_code(keycode: VirtualKeyCode) -> Option<Self> {
        match keycode {
            VirtualKeyCode::W => Some(Action::MoveForward),
            VirtualKeyCode::S => Some(Action::MoveBackward),
            VirtualKeyCode::A => Some(Action::MoveLeft),
            VirtualKeyCode::D => Some(Action::MoveRight),
            VirtualKeyCode::Space => Some(Action::MoveUp),
            VirtualKeyCode::LShift => Some(Action::MoveDown),
            _ => None,
        }
    }
}

impl InputEvent {
    pub fn from_event_state(state: ElementState, keycode: VirtualKeyCode) -> Self {
        match state {
            ElementState::Pressed => InputEvent::KeyPressed(keycode),
            ElementState::Released => InputEvent::KeyReleased(keycode),
        }
    }

    pub fn from_mouse_motion(delta_x: f64, delta_y: f64) -> Self {
        InputEvent::MouseMoved(delta_x, delta_y)
    }
}

pub struct InputManager {
    listeners: Vec<Box<dyn FnMut(InputEvent) + Send>>,
    actions: HashSet<Action>,
}

impl InputManager {
    pub fn new() -> Self {
        InputManager {
            listeners: Vec::new(),
            actions: HashSet::new(),
        }
    }

    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: FnMut(InputEvent) + 'static + Send,
    {
        self.listeners.push(Box::new(listener));
    }

    pub fn is_action_active(&self, action: &Action) -> bool {
        self.actions.contains(action)
    }

    pub fn on_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeyReleased(keycode) => {
                if let Some(action) = Action::from_key_code(keycode) {
                    self.actions.remove(&action);
                }
            }

            InputEvent::KeyPressed(keycode) => {
                if let Some(action) = Action::from_key_code(keycode) {
                    self.actions.insert(action);
                }
            }

            _ => {}
        }

        for listener in &mut self.listeners {
            listener(event.clone());
        }
    }
}
