use hecs::World;
use nalgebra_glm::{look_at, vec3};

use crate::engine::{InputManager, VoxelWorld, camera::Camera};

#[allow(dead_code)]
pub struct Engine {
    pub input_manager: InputManager,
    pub voxel: VoxelWorld,
    pub camera: Camera,
    pub world: World,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            voxel: VoxelWorld::new(),
            world: World::new(),
            camera: Camera {
                view: look_at(
                    &vec3(24.0, 24.0, 24.0), // Far outside the 16x16x16 volume
                    &vec3(0.0, 0.0, 0.0),    // Look at center
                    &vec3(0.0, 1.0, 0.0),
                ),
                camera_pos: vec3(24.0, 24.0, 24.0),
                requires_update: true,
            },
        }
    }

    pub fn init(&mut self) {}

    pub fn tick(&mut self, delta: f32) {
        self.camera.move_update(&self.input_manager, delta);
    }
}
