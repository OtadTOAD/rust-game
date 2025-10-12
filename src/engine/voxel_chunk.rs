use std::sync::Arc;

use nalgebra_glm::{TVec3, vec3};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        allocator::StandardCommandBufferAllocator,
    },
    device::Queue,
    format::Format,
    image::{ImageDimensions, StorageImage, view::ImageView},
    memory::allocator::StandardMemoryAllocator,
    sync::GpuFuture,
};

pub const CHUNK_SIZE: u8 = 16;

pub type ChunkPosition = TVec3<i32>;
pub type VoxelID = u8;

pub struct VoxelChunk {
    pub position: ChunkPosition,
    pub voxels: Vec<VoxelID>,
    pub synced: bool,

    pub gpu_image: Option<Arc<StorageImage>>,
    pub gpu_image_view: Option<Arc<ImageView<StorageImage>>>,
}

impl VoxelChunk {
    pub fn new(position: ChunkPosition) -> Self {
        let total_voxels = CHUNK_SIZE as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize;
        let voxels = vec![0u8; total_voxels];
        let synced = false;

        let gpu_image = None;
        let gpu_image_view = None;

        Self {
            position,
            voxels,
            synced,

            gpu_image_view,
            gpu_image,
        }
    }

    pub fn create_gpu_image(
        &mut self,
        allocator: &Arc<StandardMemoryAllocator>,
        queue: &Arc<Queue>,
    ) {
        if self.gpu_image.is_some() {
            return;
        }

        let image = StorageImage::new(
            allocator,
            ImageDimensions::Dim3d {
                width: CHUNK_SIZE as u32,
                height: CHUNK_SIZE as u32,
                depth: CHUNK_SIZE as u32,
            },
            Format::R8_UINT,
            Some(queue.queue_family_index()),
        )
        .unwrap();

        let view = ImageView::new_default(image.clone()).unwrap();

        self.gpu_image = Some(image);
        self.gpu_image_view = Some(view);
    }

    pub fn upload_to_gpu(
        &mut self,
        allocator: &Arc<StandardMemoryAllocator>,
        queue: Arc<Queue>,
        command: &StandardCommandBufferAllocator,
    ) {
        if self.synced {
            return;
        }

        self.create_gpu_image(allocator, &queue);

        let staging_buffer = CpuAccessibleBuffer::from_iter(
            &allocator.clone(),
            BufferUsage {
                transfer_src: true,
                ..Default::default()
            },
            false,
            self.voxels.iter().cloned(),
        )
        .unwrap();

        let image = self.gpu_image.as_ref().unwrap().clone();

        let mut builder = AutoCommandBufferBuilder::primary(
            command,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging_buffer,
                image.clone(),
            ))
            .unwrap();

        let command_buffer = builder.build().unwrap();
        let future = vulkano::sync::now(queue.device().clone())
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();
        future.wait(None).unwrap();

        self.synced = true;
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
        self.synced = false;
    }
}
