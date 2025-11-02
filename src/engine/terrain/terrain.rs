use std::collections::HashMap;

use nalgebra_glm::IVec3;

pub const CHUNK_SIZE: usize = 32;
pub const MAX_CHUNKS: usize = 64;

const GROUND_HEIGHT: usize = 8;

pub struct Chunk {
    pub voxels: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

pub struct Terrain {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl Terrain {
    pub fn new() -> Self {
        let chunks = HashMap::new();
        Terrain { chunks }
    }

    pub fn load_chunk(&mut self, chunk_pos: IVec3) {
        let mut voxels = [[[0u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    if global_y < GROUND_HEIGHT as i32 {
                        voxels[x][y][z] = 1;
                    } else {
                        voxels[x][y][z] = 0;
                    }
                }
            }
        }

        self.chunks.insert(chunk_pos, Chunk { voxels });
    }
}
