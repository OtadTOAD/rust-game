use std::{collections::HashMap, sync::Arc};

use nalgebra_glm::vec3;
use vulkano::image::{ImmutableImage, view::ImageView};

use crate::engine::{InputManager, Mesh, instance::Instance};

pub struct Engine {
    pub input_manager: InputManager,

    pub meshes: Vec<Arc<Mesh>>,
    pub textures: HashMap<usize, Arc<ImageView<ImmutableImage>>>,

    pub instances_a: HashMap<usize, Vec<Instance>>,
    pub instances_b: HashMap<usize, Vec<Instance>>,
    write_buffer: bool,
}

impl Engine {
    pub fn new() -> Engine {
        let meshes = Vec::new();
        let textures = HashMap::new();

        let instances_a = HashMap::new();
        let instances_b = HashMap::new();

        let input_manager = InputManager::new();

        Engine {
            input_manager,

            meshes,
            textures,

            write_buffer: true,
            instances_a,
            instances_b,
        }
    }

    pub fn init(&mut self) {
        // Just to make debug and release files work with debugger
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if exe_dir.ends_with("debug") || exe_dir.ends_with("release") {
                    let project_root = exe_dir.parent().unwrap().parent().unwrap();
                    let _ = std::env::set_current_dir(project_root);
                }
            }
        }

        let id = self.load_mesh("assets/meshes/Cart.glb");
        self.spawn_instance(id);
    }

    pub fn get_read_buffer(&self) -> &HashMap<usize, Vec<Instance>> {
        if self.write_buffer {
            &self.instances_b
        } else {
            &self.instances_a
        }
    }

    pub fn get_write_buffer(&mut self) -> &mut HashMap<usize, Vec<Instance>> {
        if self.write_buffer {
            &mut self.instances_a
        } else {
            &mut self.instances_b
        }
    }

    fn swap_buffers(&mut self) {
        let (write_buffer, read_buffer) = if self.write_buffer {
            (&self.instances_a, &mut self.instances_b)
        } else {
            (&self.instances_b, &mut self.instances_a)
        };

        *read_buffer = write_buffer.clone();

        self.write_buffer = !self.write_buffer;
    }

    pub fn tick(&mut self, _delta: f32) {
        let write_buffer = self.get_write_buffer();

        for (_, instances) in write_buffer {
            for inst in instances {
                inst.rotate_around_axis(_delta, vec3(0.0, 1.0, 0.0));
                inst.update_matrices();
            }
        }

        self.swap_buffers();
    }

    pub fn load_mesh(&mut self, file_path: &str) -> usize {
        let mesh = Mesh::new(file_path);
        let id = self.meshes.len();
        self.meshes.push(Arc::new(mesh));

        self.instances_a.insert(id, Vec::new());
        self.instances_b.insert(id, Vec::new());

        id
    }

    pub fn spawn_instance(&mut self, mesh_id: usize) -> usize {
        let mut inst = Instance::new();
        inst.translate(vec3(0.0, 1.5, -5.0));

        let write_buffer = self.get_write_buffer();
        let instances = write_buffer.get_mut(&mesh_id).unwrap();
        instances.push(inst);

        instances.len() - 1
    }
}
