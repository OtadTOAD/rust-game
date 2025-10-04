mod model;
mod system;

use model::Model;
use system::DirectionalLight;
use system::System;

use vulkano::sync;
use vulkano::sync::GpuFuture;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use nalgebra_glm::{look_at, pi, vec3};

use std::time::Instant;

fn main() {
    let event_loop = EventLoop::new();
    let mut system = System::new(&event_loop);

    system.set_view(&look_at(
        &vec3(0.0, 0.0, 0.1),
        &vec3(0.0, 0.0, 0.0),
        &vec3(0.0, 1.0, 0.0),
    ));

    let mut suzanne1 = Model::new("assets/meshes/Monkey.glb").build();
    suzanne1.translate(vec3(-5.0, 2.0, -8.0));

    let mut suzanne2 = Model::new("assets/meshes/Monkey.glb").build();
    suzanne2.translate(vec3(5.0, 2.0, -6.0));

    let mut suzanne3 = Model::new("assets/meshes/Monkey.glb").build();
    suzanne3.translate(vec3(0.0, -2.0, -5.0));

    let directional_light_r = DirectionalLight::new([-4.0, -4.0, 0.0, -2.0], [1.0, 0.0, 0.0]);
    let directional_light_g = DirectionalLight::new([4.0, -4.0, 0.0, -2.0], [0.0, 1.0, 0.0]);
    let directional_light_b = DirectionalLight::new([0.0, 4.0, 0.0, -2.0], [0.0, 0.0, 1.0]);

    let rotation_start = Instant::now();

    let mut previous_frame_end =
        Some(Box::new(sync::now(system.device.clone())) as Box<dyn GpuFuture>);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            system.recreate_swapchain();
        }
        Event::RedrawEventsCleared => {
            previous_frame_end
                .as_mut()
                .take()
                .unwrap()
                .cleanup_finished();

            let elapsed = rotation_start.elapsed().as_secs() as f64
                + rotation_start.elapsed().subsec_nanos() as f64 / 1_000_000_000.0;
            let elapsed_as_radians = elapsed * pi::<f64>() / 180.0;

            suzanne1.zero_rotation();
            suzanne1.rotate(elapsed_as_radians as f32 * 50.0, vec3(0.0, 0.0, 1.0));
            suzanne1.rotate(elapsed_as_radians as f32 * 30.0, vec3(0.0, 1.0, 0.0));
            suzanne1.rotate(elapsed_as_radians as f32 * 20.0, vec3(1.0, 0.0, 0.0));

            suzanne2.zero_rotation();
            suzanne2.rotate(elapsed_as_radians as f32 * 25.0, vec3(0.0, 0.0, 1.0));
            suzanne2.rotate(elapsed_as_radians as f32 * 10.0, vec3(0.0, 1.0, 0.0));
            suzanne2.rotate(elapsed_as_radians as f32 * 60.0, vec3(1.0, 0.0, 0.0));

            suzanne3.zero_rotation();
            suzanne3.rotate(elapsed_as_radians as f32 * 5.0, vec3(0.0, 0.0, 1.0));
            suzanne3.rotate(elapsed_as_radians as f32 * 45.0, vec3(0.0, 1.0, 0.0));
            suzanne3.rotate(elapsed_as_radians as f32 * 12.0, vec3(1.0, 0.0, 0.0));

            system.start();
            system.geometry(&mut suzanne1);
            system.geometry(&mut suzanne2);
            system.geometry(&mut suzanne3);
            system.ambient();
            system.directional(&directional_light_r);
            system.directional(&directional_light_g);
            system.directional(&directional_light_b);
            system.finish(&mut previous_frame_end);
        }
        _ => (),
    });
}
