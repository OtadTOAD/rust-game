use std::sync::{Arc, Mutex};

use crate::engine::{Camera, InputEvent, input::InputManager, terrain::Terrain};

pub struct Engine {
    pub terrain: Terrain,
    pub camera: Arc<Mutex<Camera>>,
    pub input: Arc<Mutex<InputManager>>,
}

impl Engine {
    pub fn new(input: Arc<Mutex<InputManager>>) -> Self {
        let camera = Arc::new(Mutex::new(Camera::new([0.0, 10.0, 0.0])));

        {
            let camera_ref = camera.clone();
            input.lock().unwrap().add_listener(move |event| {
                let mut camera_ref = camera_ref.lock().unwrap();

                const MOUSE_SENSITIVITY: f32 = 0.002;
                match event {
                    InputEvent::MouseMoved(dx, dy) => {
                        camera_ref
                            .rotate(dx as f32 * MOUSE_SENSITIVITY, dy as f32 * MOUSE_SENSITIVITY);
                    }
                    _ => {}
                }
            });
        }

        Engine {
            terrain: Terrain::new(),

            camera,
            input,
        }
    }

    pub fn tick(&mut self, delta_time: f64) {
        let mut camera = self.camera.lock().unwrap();
        let input = self.input.lock().unwrap();

        camera.tick(&input, delta_time);
    }
}
