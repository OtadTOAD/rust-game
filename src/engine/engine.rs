use std::sync::Arc;

use hecs::World;
use nalgebra_glm::{look_at, vec3};
use vulkano::image::{StorageImage, view::ImageView};

use crate::engine::{InputManager, VoxelWorld, camera::Camera};

#[allow(dead_code)]
pub struct Engine {
    pub input_manager: InputManager,
    pub voxel: VoxelWorld,
    pub camera: Camera,
    pub world: World,

    pub image: Option<Arc<StorageImage>>,
    pub image_view: Option<Arc<ImageView<StorageImage>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            voxel: VoxelWorld::new(5),
            world: World::new(),
            camera: Camera {
                view: look_at(
                    &vec3(25.0, 25.0, 25.0),
                    &vec3(0.0, 0.0, 0.0),
                    &vec3(0.0, 1.0, 0.0),
                ),
                camera_pos: vec3(25.0, 25.0, 25.0),
                requires_update: true,
            },

            image: None,
            image_view: None,
        }
    }

    pub fn init(&mut self) {
        self.voxel.load_chunks([0.0, 0.0, 0.0].into());
    }

    pub fn tick(&mut self, delta: f32) {
        self.camera.move_update(&self.input_manager, delta);
        self.voxel.load_chunks(self.camera.camera_pos);
    }
}
