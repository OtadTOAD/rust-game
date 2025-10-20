use hecs::World as EcsWorld;
use nalgebra_glm::{Vec3, look_at, vec3};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::memory::allocator::StandardMemoryAllocator;

use crate::engine::{
    InputManager,
    camera::Camera,
    chunk_manager::ChunkManager,
    unified_octree::{CHUNK_SIZE, OctreeNode, UnifiedOctree},
    world_generator::{WorldGenerator, create_sphere_world, create_terrain_world},
};

pub struct Engine {
    pub input_manager: InputManager,
    pub camera: Camera,
    pub world: EcsWorld,

    pub octree: UnifiedOctree,
    pub chunk_manager: ChunkManager,

    pub octree_buffer_dirty: bool,

    frames_since_compact: u32,
    compact_every_n_frames: u32,

    render_distance: i32,
}

impl Engine {
    pub fn new(render_distance: i32, generator: Arc<dyn WorldGenerator>) -> Self {
        let world_size = (render_distance * 2 + 1) * CHUNK_SIZE;
        let octree_size = (world_size as u32).next_power_of_two();

        println!("Initializing engine:");
        println!("  Render distance: {} chunks", render_distance);
        println!("  World size: {}^3 voxels", world_size);
        println!("  Octree size: {}^3", octree_size);
        println!("  Generator: {}", generator.name());

        let mut octree = UnifiedOctree::new(octree_size);

        let center = octree_size as i32 / 2;

        println!("  World center: ({}, {}, {})", center, center, center);

        let mut chunk_manager = ChunkManager::new(generator, render_distance);

        let max_safe_offset = (octree_size as f32 / 2.0) * 0.7;
        let camera_pos_f = vec3(
            center as f32 + max_safe_offset,
            center as f32 + max_safe_offset,
            center as f32 + max_safe_offset,
        );

        let spawn_pos = vec3(
            camera_pos_f.x.floor() as i32,
            camera_pos_f.y.floor() as i32,
            camera_pos_f.z.floor() as i32,
        );

        println!("\n=== Initial Chunk Loading ===");
        println!("Octree bounds: 0 to {}", octree_size - 1);
        println!("Octree center: ({}, {}, {})", center, center, center);
        println!(
            "Camera position: ({:.1}, {:.1}, {:.1})",
            camera_pos_f.x, camera_pos_f.y, camera_pos_f.z
        );
        println!(
            "Spawn chunk position: ({}, {}, {})",
            spawn_pos.x, spawn_pos.y, spawn_pos.z
        );
        chunk_manager.update(spawn_pos, &mut octree);

        let stats = octree.get_stats();
        println!(
            "After loading: {} nodes, {} filled leaves",
            stats.total_nodes, stats.filled_leaf_nodes
        );

        Self {
            input_manager: InputManager::new(),
            world: EcsWorld::new(),
            camera: Camera {
                view: look_at(
                    &camera_pos_f,
                    &vec3(center as f32, center as f32, center as f32),
                    &vec3(0.0, 1.0, 0.0),
                ),
                camera_pos: camera_pos_f,
                requires_update: true,
            },
            chunk_manager,
            octree,
            octree_buffer_dirty: true,
            frames_since_compact: 0,
            compact_every_n_frames: 600,
            render_distance,
        }
    }

    pub fn with_sphere(render_distance: i32, offset: Vec3, radius: f32) -> Self {
        let world_size = (render_distance * 2 + 1) * CHUNK_SIZE;
        let octree_size = (world_size as u32).next_power_of_two();
        let octree_center = octree_size as i32 / 2;

        let sphere_center = vec3(
            octree_center as f32 + offset.x,
            octree_center as f32 + offset.y,
            octree_center as f32 + offset.z,
        );

        Self::new(
            render_distance,
            create_sphere_world(sphere_center, radius).into(),
        )
    }

    pub fn with_terrain(render_distance: i32, seed: u32) -> Self {
        Self::new(render_distance, create_terrain_world(seed).into())
    }

    pub fn init(&mut self) {}

    pub fn create_octree_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<[OctreeNode]>> {
        CpuAccessibleBuffer::from_iter(
            allocator,
            BufferUsage {
                storage_buffer: true,
                ..Default::default()
            },
            false,
            self.octree.nodes.iter().cloned(),
        )
        .expect("Failed to create octree buffer")
    }

    pub fn update_gpu_buffer(&mut self, buffer: &Arc<CpuAccessibleBuffer<[OctreeNode]>>) {
        if !self.octree.needs_gpu_upload() {
            return;
        }

        if self.octree.needs_full_upload() {
            let mut write = buffer.write().unwrap();
            write.copy_from_slice(&self.octree.nodes);
            println!(
                "  Full GPU buffer update: {} nodes",
                self.octree.nodes.len()
            );
        } else {
            let ranges = self.octree.get_dirty_ranges();
            let mut write = buffer.write().unwrap();

            let mut total_updated = 0;
            let len = ranges.len();
            for (start, end) in ranges {
                let count = end - start;
                write[start..end].copy_from_slice(&self.octree.nodes[start..end]);
                total_updated += count;
            }
            println!(
                "  Partial GPU update: {} nodes in {} ranges",
                total_updated, len
            );
        }

        self.octree.clear_dirty_state();
        self.octree_buffer_dirty = false;
    }

    pub fn needs_gpu_upload(&self) -> bool {
        self.octree_buffer_dirty || self.octree.needs_gpu_upload()
    }

    pub fn tick(&mut self, delta: f32) {
        self.camera.move_update(&self.input_manager, delta);

        let camera_voxel = vec3(
            self.camera.camera_pos.x.floor() as i32,
            self.camera.camera_pos.y.floor() as i32,
            self.camera.camera_pos.z.floor() as i32,
        );

        if self.chunk_manager.update(camera_voxel, &mut self.octree) {
            self.octree_buffer_dirty = true;
        }

        self.frames_since_compact += 1;
        if self.frames_since_compact >= self.compact_every_n_frames {
            self.maybe_compact();
            self.frames_since_compact = 0;
        }
    }

    fn maybe_compact(&mut self) {
        let stats = self.octree.get_stats();
        let fragmentation = stats.free_nodes as f32 / stats.total_nodes.max(1) as f32;

        if fragmentation > 0.2 || self.octree.total_updates > 100 {
            println!("\n=== Compacting Octree ===");
            println!("  Fragmentation: {:.1}%", fragmentation * 100.0);
            println!("  Free nodes: {}", stats.free_nodes);

            let start = std::time::Instant::now();
            self.octree.compact();

            let new_stats = self.octree.get_stats();
            let saved =
                (stats.total_nodes - new_stats.total_nodes) * std::mem::size_of::<OctreeNode>();

            println!("  ✓ Compacted in {:?}", start.elapsed());
            println!(
                "  Saved: {} nodes ({:.2} MB)",
                stats.total_nodes - new_stats.total_nodes,
                saved as f32 / (1024.0 * 1024.0)
            );

            self.octree_buffer_dirty = true;
        }
    }

    pub fn place_voxel(&mut self, pos: Vec3, voxel_id: u8) {
        let voxel_pos = vec3(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        self.octree.set_voxel(voxel_pos, voxel_id);
        self.octree_buffer_dirty = true;
    }

    pub fn generate_sphere(&mut self, center: Vec3, radius: f32, voxel_id: u8) {
        let center_i = vec3(
            center.x.floor() as i32,
            center.y.floor() as i32,
            center.z.floor() as i32,
        );

        let r_i = radius.ceil() as i32;
        let min = center_i - vec3(r_i, r_i, r_i);
        let max = center_i + vec3(r_i, r_i, r_i);

        self.octree.set_region(min, max, |x, y, z| {
            let dx = (x - center_i.x) as f32;
            let dy = (y - center_i.y) as f32;
            let dz = (z - center_i.z) as f32;

            if (dx * dx + dy * dy + dz * dz).sqrt() < radius {
                voxel_id
            } else {
                0
            }
        });

        self.octree_buffer_dirty = true;
    }

    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance;
        self.chunk_manager.set_render_distance(distance);
        println!("Render distance set to: {}", distance);
    }

    pub fn get_stats(&self) -> String {
        let octree_stats = self.octree.get_stats();
        let chunk_stats = self.chunk_manager.get_stats();

        format!(
            "Nodes: {} | Mem: {:.1}MB | {} | Updates: {} | Dirty: {}",
            octree_stats.total_nodes,
            octree_stats.memory_usage() as f32 / (1024.0 * 1024.0),
            chunk_stats,
            octree_stats.total_updates,
            self.octree.needs_gpu_upload()
        )
    }

    pub fn debug_voxel_count(&self) -> (usize, usize) {
        let stats = self.octree.get_stats();
        (stats.filled_leaf_nodes, stats.leaf_nodes)
    }
}
