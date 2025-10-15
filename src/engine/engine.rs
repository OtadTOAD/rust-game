use hecs::World as EcsWorld;
use nalgebra_glm::{look_at, vec3};

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
    pub octree_needs_gpu_upload: bool,

    frames_since_last_compact: u32,
    compact_every_n_frames: u32,
    total_chunk_updates: usize,
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
            octree_needs_gpu_upload: false,

            frames_since_last_compact: 0,
            compact_every_n_frames: 600,
            total_chunk_updates: 0,
        }
    }

    pub fn get_octree(&mut self) -> Option<(&GpuOctree, &mut Octree)> {
        match (self.octree_gpu.as_ref(), self.octree.as_mut()) {
            (Some(g), Some(o)) => Some((g, o)),
            _ => None,
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
        self.octree_needs_gpu_upload = true;
        self.total_chunk_updates = 0;
        self.frames_since_last_compact = 0;
    }

    /// Unified update function that handles all chunk updates
    fn update_octree(&mut self, max_updates: usize, verbose: bool) {
        let (updates, removals) = self.voxel.get_pending_updates(max_updates);

        if updates.is_empty() && removals.is_empty() {
            return;
        }

        if let Some(octree) = &mut self.octree {
            let start = std::time::Instant::now();

            // Process removals first
            if !removals.is_empty() {
                octree.clear_chunks_batch(&removals, self.octree_offset);
                if verbose {
                    println!(
                        "  Cleared {} chunks in {:?}",
                        removals.len(),
                        start.elapsed()
                    );
                }
            }

            // Batch process updates
            for update in &updates {
                if let Some(chunk_data) = self.voxel.get_chunk_voxels(&update.position) {
                    octree.update_chunk_region(update.position, &chunk_data, self.octree_offset);
                }
            }

            self.total_chunk_updates += updates.len();

            if verbose {
                let total_time = start.elapsed();
                println!("  ✓ Updated {} chunks in {:?}", updates.len(), total_time);

                let stats = octree.get_stats();
                if stats.total_updates > 0 {
                    println!(
                        "  Cache hit rate: {:.1}% ({}/{})",
                        stats.cache_hit_rate, stats.cache_hits, stats.total_updates
                    );
                }
                println!(
                    "  Total nodes: {} (active) + {} (free)",
                    stats.total_nodes, stats.free_nodes
                );
            }

            // Update GPU octree
            if let Some(gpu_octree) = &mut self.octree_gpu {
                *gpu_octree = GpuOctree::from_octree(octree, self.octree_offset);
                self.octree_needs_gpu_upload = true;
            }
        } else {
            self.rebuild_octree();
        }
    }

    pub fn tick(&mut self, delta: f32) {
        self.camera.move_update(&self.input_manager, delta);

        let updated = self.voxel.load_chunks(self.camera.camera_pos);

        if updated && self.needs_full_rebuild() {
            self.rebuild_octree();
        } else if self.voxel.has_pending_updates() {
            let pending_count = self.voxel.pending_update_count();

            // Adaptive update batch size based on queue depth
            let (batch_size, verbose) = match pending_count {
                0 => return,
                1..=20 => (4, false),
                21..=50 => (8, false),
                _ => (16, true),
            };

            self.update_octree(batch_size, verbose);
        }

        // Periodic compaction
        self.frames_since_last_compact += 1;
        if self.frames_since_last_compact >= self.compact_every_n_frames {
            self.maybe_compact_octree();
            self.frames_since_last_compact = 0;
        }
    }

    fn maybe_compact_octree(&mut self) {
        if let Some(octree) = &mut self.octree {
            let stats = octree.get_stats();

            // Only compact if there's significant fragmentation
            let fragmentation_ratio = stats.free_nodes as f32 / stats.total_nodes.max(1) as f32;

            if fragmentation_ratio > 0.2 || self.total_chunk_updates > 100 {
                println!("\n=== Compacting Octree ===");
                println!(
                    "  Fragmentation: {:.1}% ({} free nodes)",
                    fragmentation_ratio * 100.0,
                    stats.free_nodes
                );
                println!(
                    "  Total chunk updates since last compact: {}",
                    self.total_chunk_updates
                );

                let start = std::time::Instant::now();
                octree.compact();
                let duration = start.elapsed();

                let new_stats = octree.get_stats();
                let node_size = std::mem::size_of::<crate::engine::octree::OctreeNode>();
                let bytes_saved = (stats.total_nodes - new_stats.total_nodes) * node_size;

                println!("  ✓ Compaction complete in {:?}", duration);
                println!(
                    "  Memory saved: {} bytes ({:.2} MB)",
                    bytes_saved,
                    bytes_saved as f32 / (1024.0 * 1024.0)
                );

                // Update GPU after compaction
                if let Some(gpu_octree) = &mut self.octree_gpu {
                    *gpu_octree = GpuOctree::from_octree(octree, self.octree_offset);
                    self.octree_needs_gpu_upload = true;
                }

                self.total_chunk_updates = 0;
            }
        }
    }

    fn needs_full_rebuild(&self) -> bool {
        if let Some(octree) = &self.octree {
            let (_min, _max, world_size) = self.voxel.get_world_bounds();
            let max_dim = world_size[0].max(world_size[1]).max(world_size[2]);
            let required_size = max_dim.next_power_of_two();

            octree.size < required_size
        } else {
            true
        }
    }
}
