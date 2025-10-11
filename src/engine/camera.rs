use nalgebra_glm::{TMat4, Vec3, identity, inverse, translate, vec3};

use crate::engine::InputManager;

pub struct Camera {
    pub view: TMat4<f32>,
    pub camera_pos: Vec3,
    pub requires_update: bool,
}

impl Camera {
    pub fn update_view_matrix(&mut self) {
        let mut rotation_only = identity::<f32, 4>();
        for i in 0..3 {
            for j in 0..3 {
                rotation_only[(i, j)] = self.view[(i, j)];
            }
        }

        let translation = translate(&identity(), &self.camera_pos);
        self.view = rotation_only * inverse(&translation);
    }

    fn get_forward(&self) -> Vec3 {
        vec3(-self.view[(2, 0)], -self.view[(2, 1)], -self.view[(2, 2)])
    }

    fn get_right(&self) -> Vec3 {
        vec3(self.view[(0, 0)], self.view[(0, 1)], self.view[(0, 2)])
    }

    fn get_up(&self) -> Vec3 {
        vec3(self.view[(1, 0)], self.view[(1, 1)], self.view[(1, 2)])
    }

    pub fn move_update(&mut self, input_manager: &InputManager, delta: f32) {
        let mut moved = false;
        let mut rotated = false;

        let rot_speed = 1.5 * delta;
        let world_up = vec3(0.0, 1.0, 0.0);

        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Left) {
            let rotation = nalgebra_glm::rotate_normalized_axis(&identity(), rot_speed, &world_up);
            self.view = rotation * self.view;
            rotated = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Right) {
            let rotation = nalgebra_glm::rotate_normalized_axis(&identity(), -rot_speed, &world_up);
            self.view = rotation * self.view;
            rotated = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Up) {
            let right = self.get_right();
            let rotation = nalgebra_glm::rotate_normalized_axis(&identity(), rot_speed, &right);
            self.view = rotation * self.view;
            rotated = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Down) {
            let right = self.get_right();
            let rotation = nalgebra_glm::rotate_normalized_axis(&identity(), -rot_speed, &right);
            self.view = rotation * self.view;
            rotated = true;
        }

        let forward = self.get_forward();
        let right = self.get_right();
        let up = self.get_up();

        let speed = 5.0;
        let move_dist = speed * delta;

        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::W) {
            self.camera_pos += forward * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::S) {
            self.camera_pos -= forward * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::A) {
            self.camera_pos -= right * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::D) {
            self.camera_pos += right * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::E) {
            self.camera_pos += up * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Q) {
            self.camera_pos -= up * move_dist;
            moved = true;
        }

        // Legacy Space/Shift for world up/down
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::Space) {
            self.camera_pos += vec3(0.0, 1.0, 0.0) * move_dist;
            moved = true;
        }
        if input_manager.is_key_pressed(winit::event::VirtualKeyCode::LShift) {
            self.camera_pos -= vec3(0.0, 1.0, 0.0) * move_dist;
            moved = true;
        }

        if moved || rotated {
            // Reconstruct view matrix with updated position and rotation
            self.update_view_matrix();
            self.requires_update = true;
        }
    }
}
