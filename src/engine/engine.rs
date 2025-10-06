use std::{collections::HashMap, sync::Arc};

use nalgebra_glm::{Vec3, vec3};
use vulkano::image::{ImmutableImage, view::ImageView};

use crate::engine::{Mesh, instance::Instance};

pub struct Engine {
    pub meshes: Vec<Arc<Mesh>>,
    pub textures: HashMap<usize, Arc<ImageView<ImmutableImage>>>,
    pub instances: HashMap<usize, Vec<Instance>>,
}

impl Engine {
    pub fn new() -> Engine {
        let meshes = Vec::new();
        let instances = HashMap::new();
        let textures = HashMap::new();

        Engine {
            meshes,
            instances,
            textures,
        }
    }

    pub fn init(&mut self) {
        let id = self.load_mesh("assets/meshes/Cart.glb");

        let instances_per_side = 100;
        let spacing = 0.5;
        for x in 0..instances_per_side {
            for z in 0..instances_per_side {
                let x_pos = (x as f32 - instances_per_side as f32 / 2.0) * spacing;
                let z_pos = -5.0 + (z as f32 - instances_per_side as f32 / 2.0) * spacing;
                let y_pos = 1.5;

                self.spawn_instance(id, vec3(x_pos, y_pos, z_pos));
            }
        }
    }

    pub fn tick(&mut self, _delta: f32) {
        for (_, instances) in &mut self.instances {
            for inst in instances {
                inst.update_matrices();
            }
        }
    }

    pub fn load_mesh(&mut self, file_path: &str) -> usize {
        let mesh = Mesh::new(file_path);
        let id: usize = self.meshes.len();

        self.meshes.push(Arc::new(mesh));
        self.instances.insert(id, Vec::new());

        id
    }

    pub fn spawn_instance(&mut self, mesh_id: usize, position: Vec3) -> usize {
        let inst = Instance::new(mesh_id, position);
        let instances = self.instances.get_mut(&mesh_id).unwrap();

        instances.push(inst);

        instances.len() - 1
    }
}
