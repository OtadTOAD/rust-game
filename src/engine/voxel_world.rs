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

    fn chunk_offset(pos: &ChunkPosition) -> [u32; 3] {
        [
            (pos.x * CHUNK_SIZE as i32) as u32,
            (pos.y * CHUNK_SIZE as i32) as u32,
            (pos.z * CHUNK_SIZE as i32) as u32,
        ]
    }

    pub fn create_staging_buffer_for_unsynced(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> (Arc<CpuAccessibleBuffer<[u8]>>, Vec<BufferImageCopy>) {
        let mut voxels = Vec::new();
        let mut regions = Vec::new();
        let mut buffer_offset: u64 = 0;

        for (pos, chunk) in &self.chunks {
            if !chunk.synced {
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            voxels.push(chunk.get(x, y, z));
                        }
                    }
                }

                let offset = Self::chunk_offset(pos);
                let chunk_size: u32 = CHUNK_SIZE.into();

                regions.push(BufferImageCopy {
                    buffer_offset,
                    buffer_row_length: chunk_size,
                    buffer_image_height: chunk_size,
                    image_subresource: ImageSubresourceLayers {
                        aspects: ImageAspects {
                            color: true,
                            ..Default::default()
                        },
                        mip_level: 0,
                        array_layers: 0..1,
                    },
                    image_offset: offset.into(),
                    image_extent: [chunk_size, chunk_size, chunk_size],
                    ..Default::default()
                });

                let chunk_voxel_count = (CHUNK_SIZE as u64).pow(3);
                buffer_offset += chunk_voxel_count;
            }
        }

        let staging_buffer = CpuAccessibleBuffer::from_iter(
            allocator,
            BufferUsage {
                transfer_src: true,
                ..Default::default()
            },
            false,
            voxels.iter().cloned(),
        )
        .unwrap();

        (staging_buffer, regions)
    }
}
