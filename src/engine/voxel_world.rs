use nalgebra_glm::{Vec3, vec3};
use std::{collections::HashMap, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::BufferImageCopy,
    image::{ImageAspects, ImageSubresourceLayers},
    memory::allocator::StandardMemoryAllocator,
};

use crate::engine::voxel_chunk::{CHUNK_SIZE, ChunkPosition, VoxelChunk};

pub struct VoxelWorld {
    pub chunks: HashMap<ChunkPosition, VoxelChunk>,
    pub render_dist: i16,
    last_center_chunk: Option<ChunkPosition>,
    pub needs_gpu_update: bool,
}

impl VoxelWorld {
    pub fn new(render_dist: i16) -> Self {
        Self {
            chunks: HashMap::new(),
            render_dist,
            last_center_chunk: None,
            needs_gpu_update: false,
        }
    }

    pub fn load_chunks(&mut self, pos: Vec3) -> bool {
        let main_chunk: ChunkPosition = vec3(
            (pos.x / CHUNK_SIZE as f32).floor() as i32,
            (pos.y / CHUNK_SIZE as f32).floor() as i32,
            (pos.z / CHUNK_SIZE as f32).floor() as i32,
        );

        let chunks_changed = if let Some(last) = self.last_center_chunk {
            last != main_chunk
        } else {
            true
        };

        if !chunks_changed {
            return false;
        }

        self.last_center_chunk = Some(main_chunk);
        let mut any_changes = false;

        for dx in -self.render_dist..=self.render_dist {
            for dy in -self.render_dist..=self.render_dist {
                for dz in -self.render_dist..=self.render_dist {
                    let chunk_pos = vec3(
                        main_chunk.x + dx as i32,
                        main_chunk.y + dy as i32,
                        main_chunk.z + dz as i32,
                    );

                    if !self.chunks.contains_key(&chunk_pos) {
                        let mut new_chunk = VoxelChunk::new(chunk_pos);
                        new_chunk.load();
                        self.chunks.insert(chunk_pos, new_chunk);
                        any_changes = true;
                    }
                }
            }
        }

        let chunks_to_remove: Vec<ChunkPosition> = self
            .chunks
            .keys()
            .filter(|&&pos| {
                let dx = (pos.x - main_chunk.x).abs();
                let dy = (pos.y - main_chunk.y).abs();
                let dz = (pos.z - main_chunk.z).abs();
                dx > self.render_dist as i32
                    || dy > self.render_dist as i32
                    || dz > self.render_dist as i32
            })
            .copied()
            .collect();

        for pos in chunks_to_remove {
            self.chunks.remove(&pos);
            any_changes = true;
        }

        if any_changes {
            self.needs_gpu_update = true;
        }

        any_changes
    }

    pub fn get_world_bounds(&self) -> (ChunkPosition, ChunkPosition, [u32; 3]) {
        if self.chunks.is_empty() {
            return (vec3(0, 0, 0), vec3(0, 0, 0), [0, 0, 0]);
        }

        let min_x = self.chunks.keys().map(|p| p.x).min().unwrap();
        let min_y = self.chunks.keys().map(|p| p.y).min().unwrap();
        let min_z = self.chunks.keys().map(|p| p.z).min().unwrap();

        let max_x = self.chunks.keys().map(|p| p.x).max().unwrap();
        let max_y = self.chunks.keys().map(|p| p.y).max().unwrap();
        let max_z = self.chunks.keys().map(|p| p.z).max().unwrap();

        let world_size_x = (max_x - min_x + 1) as u32 * CHUNK_SIZE as u32;
        let world_size_y = (max_y - min_y + 1) as u32 * CHUNK_SIZE as u32;
        let world_size_z = (max_z - min_z + 1) as u32 * CHUNK_SIZE as u32;

        (
            vec3(min_x, min_y, min_z),
            vec3(max_x, max_y, max_z),
            [world_size_x, world_size_y, world_size_z],
        )
    }

    pub fn create_staging_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<[u8]>> {
        let (min_pos, _max_pos, world_size) = self.get_world_bounds();

        let world_size_x = world_size[0] as usize;
        let world_size_y = world_size[1] as usize;
        let world_size_z = world_size[2] as usize;

        let total_voxels = world_size_x * world_size_y * world_size_z;
        let mut voxels = vec![0u8; total_voxels];

        let offset_x = -min_pos.x;
        let offset_y = -min_pos.y;
        let offset_z = -min_pos.z;

        for (pos, chunk) in &self.chunks {
            let chunk_offset_x = (pos.x + offset_x) as usize * CHUNK_SIZE as usize;
            let chunk_offset_y = (pos.y + offset_y) as usize * CHUNK_SIZE as usize;
            let chunk_offset_z = (pos.z + offset_z) as usize * CHUNK_SIZE as usize;

            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let world_x = chunk_offset_x + x as usize;
                        let world_y = chunk_offset_y + y as usize;
                        let world_z = chunk_offset_z + z as usize;

                        let world_idx = world_x
                            + world_y * world_size_x
                            + world_z * world_size_x * world_size_y;

                        voxels[world_idx] = chunk.get(x, y, z);
                    }
                }
            }
        }

        CpuAccessibleBuffer::from_iter(
            allocator,
            BufferUsage {
                transfer_src: true,
                ..Default::default()
            },
            false,
            voxels.iter().cloned(),
        )
        .unwrap()
    }
}
