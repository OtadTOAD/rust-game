use winit::event::{ElementState, VirtualKeyCode};

#[derive(Clone, Debug)]
pub enum InputEvent {
    KeyPressed(VirtualKeyCode),
    #[allow(dead_code)]
    KeyReleased(VirtualKeyCode),
    MouseMoved(f64, f64),
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
    listeners: Vec<Box<dyn FnMut(InputEvent)>>,
}

impl InputManager {
    pub fn new() -> Self {
        InputManager {
            listeners: Vec::new(),
        }
    }

    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: FnMut(InputEvent) + 'static,
    {
        self.listeners.push(Box::new(listener));
    }

    pub fn on_event(&mut self, event: InputEvent) {
        for listener in &mut self.listeners {
            listener(event.clone());
        }
    }
}
