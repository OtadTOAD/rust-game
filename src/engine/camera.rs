use winit::event::VirtualKeyCode;

use crate::engine::{InputEvent, input::InputListener};

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
        self.pitch += delta_pitch;

        // Clamp pitch to avoid flipping
        let pitch_limit = std::f32::consts::FRAC_PI_2 - 0.01;
        if self.pitch > pitch_limit {
            self.pitch = pitch_limit;
        } else if self.pitch < -pitch_limit {
            self.pitch = -pitch_limit;
        }

        self.is_changed = true;
    }
}

impl InputListener for Camera {
    fn on_input(&mut self, event: super::InputEvent) {
        const MOVE_SPEED: f32 = 0.5;
        const MOUSE_SENSITIVITY: f32 = 0.002;

        match event {
            InputEvent::KeyPressed(key) => match key {
                VirtualKeyCode::W => self.move_forward(MOVE_SPEED),
                VirtualKeyCode::S => self.move_backward(MOVE_SPEED),
                VirtualKeyCode::A => self.move_left(MOVE_SPEED),
                VirtualKeyCode::D => self.move_right(MOVE_SPEED),
                VirtualKeyCode::Space => self.position[1] += MOVE_SPEED,
                VirtualKeyCode::LShift => self.position[1] -= MOVE_SPEED,
                _ => {}
            },

            InputEvent::MouseMoved(delta_x, delta_y) => {
                self.rotate(
                    delta_x as f32 * MOUSE_SENSITIVITY,
                    -delta_y as f32 * MOUSE_SENSITIVITY, // Negative for natural mouse look
                );
            }
            _ => {}
        }
    }
}
