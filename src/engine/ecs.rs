use hecs::World;
use nalgebra_glm::{TMat4, Vec3, vec3};

use crate::engine::InputManager;

pub struct Transform {
    pub position: Vec3,
    pub rotation: TMat4<f32>,
    pub scale: Vec3,
}

pub struct MeshID(pub usize);

pub struct MaterialID(pub usize);

pub struct Car {
    pub velocity: Vec3,
    pub speed: f32,
    pub turn_speed: f32,
}

pub fn car_system(world: &mut World, input: &InputManager, delta: f32) {
    for (_, (car, transform)) in world.query_mut::<(&mut Car, &mut Transform)>() {
        let mut movement_input: f32 = 0.0;
        if input.is_key_pressed(winit::event::VirtualKeyCode::W) {
            movement_input += 1.0;
        }
        if input.is_key_pressed(winit::event::VirtualKeyCode::S) {
            movement_input -= 1.0;
        }

        let forward = -transform.rotation.column(2).xyz();

        let desired_velocity = forward * car.speed * movement_input;

        let current_forward_speed = car.velocity.dot(&forward);

        let accel = 20.0;
        let delta_speed = desired_velocity.dot(&forward) - current_forward_speed;

        let change = delta_speed.clamp(-accel * delta, accel * delta);

        car.velocity += forward * change;

        let lateral = car.velocity - forward * car.velocity.dot(&forward);
        car.velocity -= lateral * 0.1;

        if movement_input.abs() > 0.0 {
            let mut turn_input: f32 = 0.0;
            if input.is_key_pressed(winit::event::VirtualKeyCode::A) {
                turn_input += 1.0;
            }
            if input.is_key_pressed(winit::event::VirtualKeyCode::D) {
                turn_input -= 1.0;
            }

            if turn_input.abs() > 0.0 {
                let rotation_delta = nalgebra_glm::rotation(
                    turn_input * car.turn_speed * delta,
                    &vec3(0.0, 1.0, 0.0),
                );
                transform.rotation = rotation_delta * transform.rotation;
            }
        }

        transform.position += car.velocity * delta;
    }
}
