use std::{collections::HashMap, sync::Arc};

use nalgebra_glm::{Vec3, vec3};
use vulkano::image::{ImmutableImage, view::ImageView};

use crate::engine::{Mesh, instance::Instance};

pub struct Engine {
    pub meshes: Vec<Arc<Mesh>>,
    pub textures: HashMap<usize, Arc<ImageView<ImmutableImage>>>,
    pub instances: Vec<Instance>,
}

impl Engine {
    pub fn new() -> Engine {
        let meshes = Vec::new();
        let instances = Vec::new();
        let textures = HashMap::new();

        Engine {
            meshes,
            instances,
            textures,
        }
    }

    pub fn init(&mut self) {
        let id = self.load_mesh("assets/meshes/Cart.glb");
        self.spawn_instance(id, vec3(0.0, 1.5, -5.0));
    }

    pub fn tick(&mut self, delta: f32) {
        for instance in &mut self.instances {
            instance.rotate_around_axis(delta, vec3(0.0, 1.0, 0.0));
            instance.update_matrices();
        }
    }

    pub fn load_mesh(&mut self, file_path: &str) -> usize {
        let mesh = Mesh::new(file_path);
        self.meshes.push(Arc::new(mesh));
        self.meshes.len() - 1
    }

    pub fn spawn_instance(&mut self, mesh_id: usize, position: Vec3) -> usize {
        let instance = Instance::new(mesh_id, position);
        self.instances.push(instance);
        self.instances.len() - 1
    }
}
