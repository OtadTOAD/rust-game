use std::f32::consts::PI;

use nalgebra_glm::{
    Mat4, Quat, Vec3, infinite_perspective_rh_zo, inverse, quat_angle, quat_angle_axis,
    quat_rotate_vec3, quat_to_mat4, translation,
};

use crate::engine::input::InputManager;

const MOVE_SPEED: f32 = 15.0;
const ROTATE_SPEED: f32 = 0.01;

const VEC_Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const VEC_X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const VEC_Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);

pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub aspect_ratio: f32,

    pub is_changed: bool,

    pub inv_proj: Mat4,
    pub inv_view: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, fov_deg: f32, aspect_ratio: f32) -> Self {
        Camera {
            rotation: quat_angle_axis(PI, &VEC_Z),
            aspect_ratio,
            position,
            fov: fov_deg.to_radians(),

            is_changed: true,

            inv_proj: Mat4::identity(),
            inv_view: Mat4::identity(),
        }
    }

    /*
    pub fn forward(&self) -> Vec3 {
        quat_rotate_vec3(&self.rotation, &VEC_Z)
    }

    pub fn right(&self) -> Vec3 {
        quat_rotate_vec3(&self.rotation, &VEC_X)
    }

    pub fn up(&self) -> Vec3 {
        quat_rotate_vec3(&self.rotation, &VEC_Y)
    }
    */

    pub fn view_matrix(&self) -> Mat4 {
        let rotation = quat_to_mat4(&self.rotation.conjugate());
        let translation_mat = translation(&-self.position);
        rotation * translation_mat
    }

    pub fn proj_matrix(&self) -> Mat4 {
        infinite_perspective_rh_zo(self.aspect_ratio, self.fov, 0.1)
    }

    pub fn update_matrices(&mut self) {
        if !self.is_changed {
            return;
        }

        self.inv_proj = inverse(&self.proj_matrix());
        self.inv_view = inverse(&self.view_matrix());
        self.is_changed = false;
    }

    pub fn move_local(&mut self, dir: Vec3, len: f32) {
        let world_dir = quat_rotate_vec3(&self.rotation, &dir);
        self.position += world_dir * len;
        self.is_changed = true;
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        let q_yaw = quat_angle_axis(yaw, &VEC_Y);
        let q_pitch = quat_angle_axis(pitch, &VEC_X);
        self.rotation = q_yaw * self.rotation * q_pitch;
        self.is_changed = true;
    }

    pub fn tick(&mut self, input: &InputManager, delta_time: f64) {
        if input.is_action_active(&super::input::Action::MoveForward) {
            self.move_local(-VEC_Z, MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveBackward) {
            self.move_local(VEC_Z, MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveLeft) {
            self.move_local(-VEC_X, MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveRight) {
            self.move_local(VEC_X, MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveUp) {
            self.move_local(-VEC_Y, MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveDown) {
            self.move_local(VEC_Y, MOVE_SPEED * delta_time as f32);
        }

        println!("Camera position: {:?}", self.position);

        let (delta_x, delta_y) = input.get_mouse_delta();
        if delta_x != 0.0 || delta_y != 0.0 {
            self.rotate(delta_x as f32 * ROTATE_SPEED, delta_y as f32 * ROTATE_SPEED);
        }
    }
}
