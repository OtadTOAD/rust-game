use std::sync::{Arc, Mutex};

use nalgebra_glm::{U32Vec3, Vec3};

use crate::engine::{
    Camera,
    input::{Action, InputManager},
    model::Model,
};

pub struct Engine {
    pub input: Arc<Mutex<InputManager>>,
    pub camera: Arc<Mutex<Camera>>,
    pub models: Vec<Model>,
}

impl Engine {
    pub fn new(input: Arc<Mutex<InputManager>>, camera: Arc<Mutex<Camera>>) -> Self {
        Engine {
            models: Vec::new(),
            camera,
            input,
        }
    }

    pub fn init(&mut self) {
        let model = Model::new(U32Vec3::new(16, 16, 16), Vec3::new(0.0, 0.0, 0.0));
        self.models.push(model);

        println!("Engine initialized");
    }

    pub fn tick(&mut self, delta_time: f64) {
        let mut camera = self.camera.lock().unwrap();
        let mut input = self.input.lock().unwrap();

        {
            for model in self.models.iter_mut() {
                model.rotation *= nalgebra_glm::quat(
                    15.0f32.to_radians() * delta_time as f32,
                    15.0f32.to_radians() * delta_time as f32,
                    15.0f32.to_radians() * delta_time as f32,
                    1.0,
                );
            }
        }

        {
            if input.is_action_active(&Action::ShutDown) {
                println!("Shutting down engine...");
                std::process::exit(0);
            }

            camera.tick(&input, delta_time);
        }

        input.reset_mouse_delta();
    }
}
