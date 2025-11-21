use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{Quat, U32Vec3, Vec3};
use vulkano::{
    descriptor_set::{
        PersistentDescriptorSet, WriteDescriptorSet, allocator::StandardDescriptorSetAllocator,
    },
    device::Queue,
    format::Format,
    image::{ImageDimensions, StorageImage, view::ImageView},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{GraphicsPipeline, Pipeline},
    sampler::Sampler,
};

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Default)]
pub struct DrawModel {
    pub in_model: [[f32; 4]; 4],
    pub in_model_inv: [[f32; 4]; 4],
    pub in_model_inv_pose: [[f32; 4]; 4],
}

fn get_draw_model_matrix(position: &Vec3, size: &U32Vec3, rotation: &Quat) -> DrawModel {
    let translation = nalgebra_glm::translation(position);
    let scale = nalgebra_glm::scaling(&Vec3::new(size.x as f32, size.y as f32, size.z as f32));

    let model_matrix = translation * nalgebra_glm::quat_to_mat4(rotation) * scale;

    DrawModel {
        in_model: model_matrix.into(),
        in_model_inv: model_matrix.try_inverse().unwrap().into(),
        in_model_inv_pose: model_matrix.try_inverse().unwrap().transpose().into(),
    }
}

pub struct Model {
    pub size: U32Vec3,
    //pub position: Vec3,
    pub rotation: Quat,
    pub voxels: Vec<u8>,

    pub voxel_texture: Option<Arc<ImageView<StorageImage>>>,
    pub voxel_set: Option<Arc<PersistentDescriptorSet>>,

    pub _draw: DrawModel,

    pub is_initialized: bool,
    pub is_dirty: bool,
}

impl Model {
    pub fn new(size: U32Vec3, position: Vec3) -> Self {
        let mut voxels = vec![0; (size.x * size.y * size.z) as usize];

        let mid = Vec3::new(
            size.x as f32 / 2.0,
            size.y as f32 / 2.0,
            size.z as f32 / 2.0,
        );
        let r = size.x.min(size.y).min(size.z) as f32 / 2.0;

        for x in 0..size.x as usize {
            for y in 0..size.y as usize {
                for z in 0..size.z as usize {
                    let pos = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                    let offset = pos - mid;
                    let val = if offset.magnitude() <= r { 1u8 } else { 0u8 };
                    let idx = x + y * (size.x as usize) + z * (size.x as usize) * (size.y as usize);
                    voxels[idx] = val;
                }
            }
        }

        let rotation = Quat::identity();

        Model {
            rotation,
            //position,
            voxels,
            size,

            voxel_texture: None,
            voxel_set: None,

            _draw: get_draw_model_matrix(&position, &size, &rotation),

            is_initialized: false,
            is_dirty: false,
        }
    }

    pub fn init_model(
        &mut self,
        pipeline: Arc<GraphicsPipeline>,
        set_allocator: &StandardDescriptorSetAllocator,
        mem_allocator: &StandardMemoryAllocator,
        sampler: Arc<Sampler>,
        queue: Arc<Queue>,
    ) {
        let voxel_image = StorageImage::new(
            mem_allocator,
            ImageDimensions::Dim3d {
                width: self.size.x,
                height: self.size.y,
                depth: self.size.z,
            },
            Format::R8_UINT,
            [queue.queue_family_index()],
        )
        .unwrap();

        let voxel_layout = pipeline.layout().set_layouts().get(1).unwrap();
        let voxel_set = PersistentDescriptorSet::new(
            set_allocator,
            voxel_layout.clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                ImageView::new_default(voxel_image.clone()).unwrap(),
                sampler.clone(),
            )],
        )
        .unwrap();

        self.voxel_texture = Some(ImageView::new_default(voxel_image.clone()).unwrap());
        self.voxel_set = Some(voxel_set);
        self.is_initialized = true;
        self.is_dirty = true;
    }

    pub fn get_draw(&self) -> DrawModel {
        self._draw.clone()
    }
}
