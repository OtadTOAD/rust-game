use nalgebra_glm::Mat3x2;

use crate::engine::input::InputManager;

const MOVE_SPEED: f32 = 25.0;

pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,

    pub is_changed: bool,
}

impl Camera {
    pub fn new(pos: [f32; 3]) -> Self {
        Camera {
            position: pos,
            yaw: 0.0,
            pitch: 0.0,
            fov: 70.0_f32.to_radians(),

            is_changed: true,
        }
    }

    pub fn forward(&self) -> [f32; 3] {
        [
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ]
    }

    pub fn right(&self) -> [f32; 3] {
        [
            (self.yaw + std::f32::consts::FRAC_PI_2).cos(),
            0.0,
            (self.yaw + std::f32::consts::FRAC_PI_2).sin(),
        ]
    }

    pub fn move_forward(&mut self, distance: f32) {
        let forward = self.forward();

        self.position[0] += forward[0] * distance;
        self.position[1] += forward[1] * distance;
        self.position[2] += forward[2] * distance;

        self.is_changed = true;
    }

    pub fn move_backward(&mut self, distance: f32) {
        self.move_forward(-distance);
    }

    pub fn move_right(&mut self, distance: f32) {
        let right = self.right();
        self.position[0] += right[0] * distance;
        self.position[2] += right[2] * distance;

        self.is_changed = true;
    }

    pub fn move_left(&mut self, distance: f32) {
        self.move_right(-distance);
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch -= delta_pitch;

        // Clamp pitch to avoid flipping
        let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01;
        if self.pitch > pitch_limit {
            self.pitch = pitch_limit;
        } else if self.pitch < -pitch_limit {
            self.pitch = -pitch_limit;
        }

        self.is_changed = true;
    }

    pub fn tick(&mut self, input: &InputManager, delta_time: f64) -> Mat3x2 {
        if input.is_action_active(&super::input::Action::MoveForward) {
            self.move_forward(MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveBackward) {
            self.move_backward(MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveLeft) {
            self.move_left(MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveRight) {
            self.move_right(MOVE_SPEED * delta_time as f32);
        }
        if input.is_action_active(&super::input::Action::MoveUp) {
            self.position[1] += MOVE_SPEED * delta_time as f32;
            self.is_changed = true;
        }
        if input.is_action_active(&super::input::Action::MoveDown) {
            self.position[1] -= MOVE_SPEED * delta_time as f32;
            self.is_changed = true;
        }

        Mat3x2::new(
            self.position[0],
            self.position[1],
            self.position[2],
            self.yaw,
            self.pitch,
            self.fov,
        )
    }
}
