#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{Mat4, TMat4, TVec3, Vec3, identity, pi, rotate_normalized_axis, vec3};
use once_cell::sync::Lazy;

pub struct Instance {
    pub model_id: usize,

    pub position: Vec3,
    pub rotation: TMat4<f32>,
    pub scale: Vec3,

    pub model_matrix: TMat4<f32>,
    pub normal_matrix: TMat4<f32>,

    pub requires_update: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod, Default)]
pub struct DrawInstance {
    // This is struct used to pass in instance transformation for instancing
    pub instance_model: [[f32; 4]; 4],
    pub instance_normal: [[f32; 4]; 4],
}

impl DrawInstance {
    pub fn new(instance_model: [[f32; 4]; 4], instance_normal: [[f32; 4]; 4]) -> Self {
        Self {
            instance_model,
            instance_normal,
        }
    }
}

static DEFAULT_ROTATION: Lazy<TMat4<f32>> = Lazy::new(|| {
    let default = identity();
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 0.0, 1.0));
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 1.0, 0.0));

    default
});

impl Instance {
    pub fn new(model_id: usize, position: Vec3) -> Self {
        let instance = Instance {
            model_id,

            position,
            rotation: DEFAULT_ROTATION.clone(),
            scale: vec3(1.0, 1.0, 1.0),

            model_matrix: identity(),
            normal_matrix: identity(),

            requires_update: true,
        };
        instance
    }

    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
        self.requires_update = true;
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.requires_update = true;
    }

    pub fn rotate_around_axis(&mut self, radians: f32, axis: TVec3<f32>) {
        self.rotation = rotate_normalized_axis(&self.rotation, radians, &axis);
        self.requires_update = true;
    }

    pub fn rotate(&mut self, rotation: Mat4) {
        self.rotation = rotation * self.rotation;
        self.requires_update = true;
    }

    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.requires_update = true;
    }

    pub fn update_matrices(&mut self) {
        if !self.requires_update {
            return;
        }

        let translation = nalgebra_glm::translation(&self.position);
        let scale = nalgebra_glm::scaling(&self.scale);

        self.model_matrix = translation * self.rotation * scale;
        self.normal_matrix = nalgebra_glm::inverse_transpose(self.model_matrix);
        self.requires_update = false;
    }

    pub fn model_matrices(&self) -> (TMat4<f32>, TMat4<f32>) {
        (self.model_matrix, self.normal_matrix)
    }
}
