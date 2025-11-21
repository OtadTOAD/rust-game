use nalgebra_glm::IVec3;

pub const CHUNK_SIZE: usize = 16;
const FLOOR_HEIGHT: i32 = 8;

pub fn generate_chunk_voxels(pos: IVec3) -> Vec<u8> {
    let mut voxels = vec![0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];

    let world_y_offset = pos.y * CHUNK_SIZE as i32;

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_y = world_y_offset + y as i32;
                let index = (x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE) as usize;

                if world_y <= FLOOR_HEIGHT {
                    voxels[index] = 1;
                } else {
                    voxels[index] = 0;
                }
            }
        }
    }

    voxels
}
