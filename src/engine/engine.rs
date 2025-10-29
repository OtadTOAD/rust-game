use std::sync::{Arc, Mutex};

use winit::event::VirtualKeyCode;

use crate::engine::{Camera, InputEvent, input::InputManager, terrain::Terrain};

pub struct Engine {
    pub terrain: Terrain,
    pub camera: Arc<Mutex<Camera>>,
    pub input: InputManager,
}

impl Engine {
    pub fn new() -> Self {
        let mut input = InputManager::new();
        let camera = Arc::new(Mutex::new(Camera::new([0.0, 10.0, 0.0])));

        let camera_ref = camera.clone();
        input.add_listener(move |event| {
            let mut camera_ref = camera_ref.lock().unwrap();

            const MOVE_SPEED: f32 = 0.5;
            const MOUSE_SENSITIVITY: f32 = 0.002;
            println!("UPDATE: {:?}", event);
            match event {
                InputEvent::KeyPressed(key) => match key {
                    VirtualKeyCode::W => camera_ref.move_forward(MOVE_SPEED),
                    VirtualKeyCode::S => camera_ref.move_backward(MOVE_SPEED),
                    VirtualKeyCode::A => camera_ref.move_left(MOVE_SPEED),
                    VirtualKeyCode::D => camera_ref.move_right(MOVE_SPEED),
                    VirtualKeyCode::Space => camera_ref.position[1] += MOVE_SPEED,
                    VirtualKeyCode::LShift => camera_ref.position[1] -= MOVE_SPEED,
                    _ => {}
                },
                InputEvent::MouseMoved(dx, dy) => {
                    camera_ref.rotate(dx as f32 * MOUSE_SENSITIVITY, dy as f32 * MOUSE_SENSITIVITY);
                }
                _ => {}
            }
        });

        Engine {
            terrain: Terrain::new(),

            camera,
            input,
        }
    }
}
