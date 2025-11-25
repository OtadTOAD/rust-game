use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use nalgebra_glm::{IVec3, U32Vec3, Vec3};
use noise::Perlin;
use vulkano::sync::GpuFuture;

use crate::{
    engine::{
        Camera, DrawModel,
        input::{Action, InputManager},
        model::Model,
        terrain::{CHUNK_SIZE, generate_chunk_voxels},
    },
    render::Render,
};

pub struct Engine {
    pub input: Arc<Mutex<InputManager>>,
    pub camera: Arc<Mutex<Camera>>,

    pub chunks: HashMap<IVec3, Option<Model>>,
    pub perlin: Perlin,
}

const RENDER_CHUNK_DISTANCE: i32 = 4;
const RENDER_CHUNK_DISTANCE_Y: (i32, i32) = (2, 1);

impl Engine {
    pub fn new(input: Arc<Mutex<InputManager>>, camera: Arc<Mutex<Camera>>) -> Self {
        Engine {
            camera,
            input,

            chunks: HashMap::new(),
            perlin: Perlin::new(0),
        }
    }

    pub fn init(&mut self) {
        /*
        let model = Model::new(U32Vec3::new(16, 16, 16), Vec3::new(0.0, 0.0, 0.0));
        let model2: Model = Model::new(U32Vec3::new(8, 8, 8), Vec3::new(15.0, 0.0, 0.0));
        self.models.push(model);
        self.models.push(model2);
        */

        println!("Engine initialized");
    }

    pub fn pre_render_tick(
        &mut self,
        render: &mut Render,
        prev_frame_end: &mut Option<Box<dyn GpuFuture>>,
    ) {
        let mut dirty_models = vec![];

        let mut init_budget = 2;
        let dirty_budget = 2;

        for model in self.get_mut_models() {
            if let Some(model) = model {
                if !model.is_initialized && init_budget > 0 {
                    render.init_model(model);
                    init_budget -= 1;
                }
                if model.is_dirty && dirty_models.len() <= dirty_budget {
                    dirty_models.push(model);
                }
            }
        }

        if !dirty_models.is_empty() {
            render.update_models(&mut dirty_models, prev_frame_end);
        }
    }

    pub fn get_mut_models(&mut self) -> Vec<&mut Option<Model>> {
        self.chunks.values_mut().collect()
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

            let mut visible_chunks = HashSet::new();
            for x in -RENDER_CHUNK_DISTANCE..=RENDER_CHUNK_DISTANCE {
                for y in -RENDER_CHUNK_DISTANCE_Y.0..=RENDER_CHUNK_DISTANCE_Y.1 {
                    for z in -RENDER_CHUNK_DISTANCE..=RENDER_CHUNK_DISTANCE {
                        let chunk_pos = origin_pos + IVec3::new(x, y, z);
                        visible_chunks.insert(chunk_pos);
                        if !self.chunks.contains_key(&chunk_pos) {
                            // Still mark the chunk as generated even if it's empty
                            // To prevent regenerating it every frame
                            self.chunks.insert(chunk_pos, None);

                            let chunk = generate_chunk_voxels(chunk_pos, &self.perlin);
                            if chunk.is_none() {
                                continue;
                            }

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
                            model.voxels = chunk.unwrap();
                            self.chunks.insert(chunk_pos, Some(model));
                        }
                    }
                }
            }

            self.chunks
                .retain(|chunk_pos, _| visible_chunks.contains(chunk_pos));
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
