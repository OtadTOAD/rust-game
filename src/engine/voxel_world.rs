use std::sync::Arc;

use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    memory::allocator::StandardMemoryAllocator,
};

pub struct VoxelWorld {
    pub size: [u32; 3],
    pub voxels: Vec<u8>,
}

impl VoxelWorld {
    pub fn new() -> Self {
        let size = [16, 16, 16];
        let mut voxels = vec![0u8; (size[0] * size[1] * size[2]) as usize];

        let cx = size[0] as f32 / 2.0;
        let cy = size[1] as f32 / 2.0;
        let cz = size[2] as f32 / 2.0;
        let radius = 6.0;

        for z in 0..size[2] {
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let idx = (x + y * size[0] + z * size[0] * size[1]) as usize;

                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dz = z as f32 - cz;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    if distance < radius {
                        voxels[idx] = 200;
                    }
                }
            }
        }

        Self { size, voxels }
    }
    pub fn create_staging_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<[u8]>> {
        CpuAccessibleBuffer::from_iter(
            allocator,
            BufferUsage {
                transfer_src: true,
                ..Default::default()
            },
            false,
            self.voxels.iter().cloned(),
        )
        .unwrap()
    }

    #[allow(dead_code)]
    pub fn update_staging_buffer(&self, buffer: &Arc<CpuAccessibleBuffer<[u8]>>) {
        let mut write_lock = buffer.write().unwrap();
        write_lock.copy_from_slice(&self.voxels);
    }

    #[allow(dead_code)]
    pub fn get_index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + y * self.size[0] + z * self.size[0] * self.size[1]) as usize
    }

    #[allow(dead_code)]
    pub fn get(&self, x: u32, y: u32, z: u32) -> u8 {
        self.voxels[self.get_index(x, y, z)]
    }
}
