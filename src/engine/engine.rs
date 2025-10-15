use std::sync::Arc;

use hecs::World as EcsWorld;
use nalgebra_glm::{look_at, vec3};
use vulkano::image::{StorageImage, view::ImageView};

use crate::engine::{
    InputManager, VoxelWorld, camera::Camera, octree::Octree, octree_builder::OctreeBuilder,
    octree_gpu::GpuOctree,
};

pub struct Engine {
    pub input_manager: InputManager,
    pub voxel: VoxelWorld,
    pub camera: Camera,
    pub world: EcsWorld,

    pub octree: Option<Octree>,
    pub octree_offset: [i32; 3],
    pub octree_gpu: Option<GpuOctree>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            voxel: VoxelWorld::new(5),
            world: EcsWorld::new(),
            camera: Camera {
                view: look_at(
                    &vec3(25.0, 25.0, 25.0),
                    &vec3(0.0, 0.0, 0.0),
                    &vec3(0.0, 1.0, 0.0),
                ),
                camera_pos: vec3(25.0, 25.0, 25.0),
                requires_update: true,
            },

            octree: None,
            octree_offset: [0, 0, 0],
            octree_gpu: None,
        }
    }

    pub fn init(&mut self) {
        self.voxel.load_chunks([0.0, 0.0, 0.0].into());
        self.rebuild_octree();
    }

    pub fn rebuild_octree(&mut self) {
        println!("\n=== Building Octree ===");

        let (octree, offset) = OctreeBuilder::from_voxel_world(&self.voxel);

        if let Err(e) = OctreeBuilder::verify_octree(&self.voxel, &octree, offset) {
            eprintln!("ERROR: Octree verification failed: {}", e);
        }

        let stats = octree.get_stats();
        let (_min, _max, size) = self.voxel.get_world_bounds();
        OctreeBuilder::print_stats(&stats, size);

        let gpu_octree = GpuOctree::from_octree(&octree, offset);

        self.octree = Some(octree);
        self.octree_offset = offset;
        self.octree_gpu = Some(gpu_octree);
    }

    pub fn tick(&mut self, delta: f32) {
        self.camera.move_update(&self.input_manager, delta);

        let updated = self.voxel.load_chunks(self.camera.camera_pos);
        if updated {
            print!("REBUILDING!");
            self.rebuild_octree();
        }
    }
}
