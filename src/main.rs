mod engine;
mod system;

use system::DirectionalLight;
use system::System;

use vulkano::sync;
use vulkano::sync::GpuFuture;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use nalgebra_glm::{look_at, pi, vec3};

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use crate::engine::Engine;

const ENGINE_TICK_RATE: f32 = 60.0;

fn main() {
    let event_loop = EventLoop::new();
    let mut system = System::new(&event_loop);

    // Made this Arc/Mutex because both rendering loop and tick loop need access of this obj
    let engine = Arc::new(Mutex::new(Engine::new()));

    system.set_view(&look_at(
        &vec3(0.0, 0.0, 0.1),
        &vec3(0.0, 0.0, 0.0),
        &vec3(0.0, 1.0, 0.0),
    ));

    {
        let mut e = engine.lock().unwrap();
        e.init();

        system.preload_textures(&mut e);
    }

    let rotation_start = Instant::now();

    let mut last_frame_time = Instant::now();

    let mut previous_frame_end =
        Some(Box::new(sync::now(system.device.clone())) as Box<dyn GpuFuture>);

    let engine_for_tick = engine.clone();
    thread::spawn(move || {
        let timestep = 1.0 / ENGINE_TICK_RATE;
        loop {
            {
                let mut e = engine_for_tick.lock().unwrap();
                e.tick(timestep);
            }

            std::thread::sleep(std::time::Duration::from_secs_f32(timestep));
        }
    });

    let engine_for_render = engine.clone();
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

            let current_time = Instant::now();
            let frame_time = current_time.duration_since(last_frame_time);
            last_frame_time = current_time;
            let fps = 1.0 / frame_time.as_secs_f32();
            println!(
                "Frame time: {:.2}ms | FPS: {:.1}",
                frame_time.as_secs_f32() * 1000.0,
                fps
            );

            let elapsed = rotation_start.elapsed().as_secs() as f32
                + rotation_start.elapsed().subsec_nanos() as f32 / 1_000_000_000.0;
            let elapsed_as_radians = elapsed * 30.0 * (pi::<f32>() / 180.0);

            let x: f32 = 2.0 * elapsed_as_radians.cos();
            let z: f32 = -3.0 + (2.0 * elapsed_as_radians.sin());

            let directional_light = DirectionalLight::new([x, 0.0, z, 1.0], [1.0, 1.0, 1.0]);

            system.start();

            let e = engine_for_render.lock().unwrap();
            for instance in &e.instances {
                let tex = Arc::clone(e.textures.get(&instance.model_id).unwrap());
                let mesh = Arc::clone(&e.meshes[instance.model_id]);
                system.geometry(instance, tex, mesh);
            }

            system.ambient();
            system.directional(&directional_light);
            system.finish(&mut previous_frame_end);
        }
        _ => (),
    });
}
