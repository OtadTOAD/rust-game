use nalgebra_glm::IVec3;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 16;
const FLOOR_HEIGHT: i32 = 0;
const PERLIN_SCALE: f64 = 0.05;

pub fn generate_chunk_voxels(pos: IVec3, perlin: &Perlin) -> Option<Vec<u8>> {
    let mut voxels = vec![0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];

    let world_x_offset = pos.x * CHUNK_SIZE as i32;
    let world_y_offset = pos.y * CHUNK_SIZE as i32;
    let world_z_offset = pos.z * CHUNK_SIZE as i32;

    let mut is_empty = true;
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_x = world_x_offset + x as i32;
                let world_z = world_z_offset + z as i32;
                let world_y = world_y_offset + y as i32;
                let index = (x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE) as usize;

                let val = perlin.get([
                    world_x as f64 * PERLIN_SCALE,
                    0.0,
                    world_z as f64 * PERLIN_SCALE,
                ]);
                let height = FLOOR_HEIGHT + (val * 10.0) as i32;

                if world_y <= height {
                    voxels[index] = 1;
                    is_empty = false;
                } else {
                    voxels[index] = 0;
                }
            }
        }
    }

    if is_empty { None } else { Some(voxels) }
}
