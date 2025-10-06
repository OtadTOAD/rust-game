mod engine;
mod system;

use system::DirectionalLight;
use system::System;

use vulkano::sync;
use vulkano::sync::GpuFuture;

use winit::event::ElementState;
use winit::event::KeyboardInput;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use nalgebra_glm::{look_at, vec3};

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::engine::Engine;

const ENGINE_TICK_RATE: f32 = 60.0;

fn main() {
    let event_loop = EventLoop::new();
    let mut system = System::new(&event_loop);

    // Made this Arc/Mutex because both rendering loop and tick loop need access of this obj
    let engine = Arc::new(Mutex::new(Engine::new()));

    system.set_view(&look_at(
        &vec3(0.0, -1.5, 0.1),
        &vec3(0.0, -1.5, 0.0),
        &vec3(0.0, 1.0, 0.0),
    ));

    {
        let mut e = engine.lock().unwrap();
        e.init();

        system.preload_textures(&mut e);
    }

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

    let sun_light = DirectionalLight::new([100.0, -100.0, 100.0, 1.0], [1.0, 1.0, 1.0]);

    let engine_for_render = engine.clone();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let mut e = engine_for_render.lock().unwrap();
                match state {
                    ElementState::Pressed => e.input_manager.press_key(keycode),
                    ElementState::Released => e.input_manager.release_key(keycode),
                }
            }
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::Resized(_) => {
                system.recreate_swapchain();
            }
            _ => {}
        },
        Event::RedrawEventsCleared => {
            previous_frame_end
                .as_mut()
                .take()
                .unwrap()
                .cleanup_finished();

            let mut e = engine_for_render.lock().unwrap();

            if e.camera.requires_update {
                e.camera.requires_update = false;
                system.set_view(&e.camera.view);
            }

            system.start();

            let draw_calls = e.get_draw_calls();
            for (mesh_id, instances) in draw_calls {
                let tex = Arc::clone(e.textures.get(&mesh_id).unwrap());
                let mesh = Arc::clone(&e.meshes[mesh_id]);
                system.geometry(instances, tex, mesh);
            }

            system.ambient();
            system.directional(&sun_light);
            system.finish(&mut previous_frame_end);
        }
        _ => (),
    });
}
