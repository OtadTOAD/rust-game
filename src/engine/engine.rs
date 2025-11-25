use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use nalgebra_glm::{IVec3, U32Vec3, Vec3};

use crate::engine::{
    Camera,
    input::{Action, InputManager},
    model::Model,
    terrain::{CHUNK_SIZE, generate_chunk_voxels},
};

pub struct Engine {
    pub input: Arc<Mutex<InputManager>>,
    pub camera: Arc<Mutex<Camera>>,
    pub models: Vec<Model>,
    pub chunks: HashSet<IVec3>,
}

const RENDER_CHUNK_DISTANCE: i32 = 4;
const RENDER_CHUNK_DISTANCE_Y: (i32, i32) = (2, 1);

impl Engine {
    pub fn new(input: Arc<Mutex<InputManager>>, camera: Arc<Mutex<Camera>>) -> Self {
        Engine {
            models: Vec::new(),
            camera,
            input,
            chunks: HashSet::new(),
        }
    }

    pub fn init(&mut self) {
        let model = Model::new(U32Vec3::new(16, 16, 16), Vec3::new(0.0, 0.0, 0.0));
        let model2: Model = Model::new(U32Vec3::new(8, 8, 8), Vec3::new(15.0, 0.0, 0.0));
        self.models.push(model);
        self.models.push(model2);

        println!("Engine initialized");
    }

    pub fn tick(&mut self, delta_time: f64) {
        let mut camera = self.camera.lock().unwrap();
        let mut input = self.input.lock().unwrap();

        {
            let origin_pos = IVec3::new(
                (camera.position.x as f32 / CHUNK_SIZE as f32).round() as i32,
                (camera.position.y as f32 / CHUNK_SIZE as f32).round() as i32,
                (camera.position.z as f32 / CHUNK_SIZE as f32).round() as i32,
            );

            for x in -RENDER_CHUNK_DISTANCE..=RENDER_CHUNK_DISTANCE {
                for y in -RENDER_CHUNK_DISTANCE_Y.0..=RENDER_CHUNK_DISTANCE_Y.1 {
                    for z in -RENDER_CHUNK_DISTANCE..=RENDER_CHUNK_DISTANCE {
                        let chunk_pos = origin_pos + IVec3::new(x, y, z);
                        if !self.chunks.contains(&chunk_pos) {
                            self.chunks.insert(chunk_pos);

                            let mut model = Model::new(
                                U32Vec3::new(
                                    CHUNK_SIZE as u32,
                                    CHUNK_SIZE as u32,
                                    CHUNK_SIZE as u32,
                                ),
                                Vec3::new(
                                    chunk_pos.x as f32 * CHUNK_SIZE as f32,
                                    chunk_pos.y as f32 * CHUNK_SIZE as f32,
                                    chunk_pos.z as f32 * CHUNK_SIZE as f32,
                                ),
                            );
                            model.voxels = generate_chunk_voxels(chunk_pos);
                            self.models.push(model);
                        }
                    }
                }
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
