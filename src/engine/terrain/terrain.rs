const CHUNK_SIZE: usize = 16;

const GROUND_HEIGHT: usize = 8;

pub struct Terrain {
    pub voxels: [[[u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl Terrain {
    pub fn new() -> Self {
        let mut voxels = [[[0u8; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    voxels[x][y][z] = if y < GROUND_HEIGHT { 1 } else { 0 };
                }
            }
        }

        Terrain { voxels }
    }
}
