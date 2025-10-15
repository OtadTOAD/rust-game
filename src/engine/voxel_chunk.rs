// voxel_chunk.rs - Modified with update tracking

use nalgebra_glm::{TVec3, vec3};

pub const CHUNK_SIZE: u8 = 16;

pub type ChunkPosition = TVec3<i32>;
pub type VoxelID = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkUpdateType {
    Added,
    Modified,
    Removed,
}

pub struct VoxelChunk {
    pub position: ChunkPosition,
    pub voxels: Vec<VoxelID>,
    pub update_type: Option<ChunkUpdateType>,
}

impl VoxelChunk {
    pub fn new(position: ChunkPosition) -> Self {
        let total_voxels = CHUNK_SIZE as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize;
        let voxels = vec![0u8; total_voxels];

        Self {
            position,
            voxels,
            update_type: Some(ChunkUpdateType::Added),
        }
    }

    pub fn load(&mut self) {
        let world_origin = vec3(
            self.position.x * CHUNK_SIZE as i32,
            self.position.y * CHUNK_SIZE as i32,
            self.position.z * CHUNK_SIZE as i32,
        );

        let sphere_center = vec3(0, 0, 0);
        let radius = 100.0f32;

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let world_x = world_origin.x + x as i32;
                    let world_y = world_origin.y + y as i32;
                    let world_z = world_origin.z + z as i32;

                    let dx = world_x as f32 - sphere_center.x as f32;
                    let dy = world_y as f32 - sphere_center.y as f32;
                    let dz = world_z as f32 - sphere_center.z as f32;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    let mut id = 0;
                    if distance < radius {
                        id = 1;
                    }

                    self.set(x, y, z, id);
                }
            }
        }

        self.update_type = Some(ChunkUpdateType::Added);
    }

    #[inline]
    fn get_idx(x: u8, y: u8, z: u8) -> usize {
        let chunk_size = CHUNK_SIZE as usize;
        x as usize + y as usize * chunk_size + z as usize * chunk_size * chunk_size
    }

    pub fn get(&self, x: u8, y: u8, z: u8) -> VoxelID {
        self.voxels[Self::get_idx(x, y, z)]
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, val: VoxelID) {
        let idx = Self::get_idx(x, y, z);
        self.voxels[idx] = val;

        if self.update_type != Some(ChunkUpdateType::Added) {
            self.update_type = Some(ChunkUpdateType::Modified);
        }
    }

    pub fn mark_update_processed(&mut self) {
        self.update_type = None;
    }

    pub fn needs_update(&self) -> bool {
        self.update_type.is_some()
    }
}
