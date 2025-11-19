use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{Quat, U32Vec3, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Default)]
pub struct DrawModel {
    pub in_model: [[f32; 4]; 4],
    pub in_model_inv: [[f32; 4]; 4],
    pub in_model_inv_pose: [[f32; 4]; 4],
}

pub struct Model {
    pub size: U32Vec3,
    pub position: Vec3,
    pub rotation: Quat,
    pub voxels: Vec<u8>,
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

        Model {
            rotation: Quat::identity(),
            position,
            voxels,
            size,
        }
    }

    pub fn get_draw(&self) -> DrawModel {
        let translation = nalgebra_glm::translation(&self.position);
        let scale = nalgebra_glm::scaling(&Vec3::new(
            self.size.x as f32,
            self.size.y as f32,
            self.size.z as f32,
        ));

        let model_matrix = translation * nalgebra_glm::quat_to_mat4(&self.rotation) * scale;
        // TODO: Precompute these values instead of calcuating them every frame
        DrawModel {
            in_model: model_matrix.into(),
            in_model_inv: model_matrix.try_inverse().unwrap().into(),
            in_model_inv_pose: model_matrix.try_inverse().unwrap().transpose().into(),
        }
    }
}
