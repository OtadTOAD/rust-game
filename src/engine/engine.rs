use std::{collections::HashMap, sync::Arc};

use hecs::{Entity, World};
use nalgebra_glm::{TMat4, Vec3, identity, pi, rotate_normalized_axis, vec3};
use once_cell::sync::Lazy;
use vulkano::image::{ImmutableImage, view::ImageView};

use crate::engine::{
    DrawInstance, InputManager, Mesh,
    ecs::{Car, MeshID, Transform, car_system},
};

static DEFAULT_ROTATION: Lazy<TMat4<f32>> = Lazy::new(|| {
    let default = identity();
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 0.0, 1.0));
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 1.0, 0.0));

    default
});

pub struct Engine {
    pub input_manager: InputManager,
    pub meshes: Vec<Arc<Mesh>>,
    pub textures: HashMap<usize, Arc<ImageView<ImmutableImage>>>,
    pub world: World,

    car_entity: Option<Entity>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            textures: HashMap::new(),
            world: World::new(),
            meshes: Vec::new(),

            car_entity: None,
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

        let car_mesh_id = self.load_mesh("assets/meshes/Cart.glb");
        self.spawn_car(car_mesh_id, vec3(0.0, 0.0, -6.0));
        let goal_mesh_id = self.load_mesh("assets/meshes/Goal.glb");
        self.spawn_instance(goal_mesh_id, vec3(0.0, 0.0, -15.0));
    }

    pub fn load_mesh(&mut self, file_path: &str) -> usize {
        let mesh = Mesh::new(file_path);
        let id = self.meshes.len();
        self.meshes.push(Arc::new(mesh));
        id
    }

    pub fn spawn_instance(&mut self, mesh_id: usize, pos: Vec3) -> Entity {
        self.world.spawn((
            Transform {
                position: pos,
                rotation: DEFAULT_ROTATION.clone(),
                scale: vec3(1.0, 1.0, 1.0),
            },
            MeshID(mesh_id),
        ))
    }

    pub fn spawn_car(&mut self, mesh_id: usize, pos: Vec3) {
        let entity = self.world.spawn((
            Transform {
                position: pos,
                rotation: DEFAULT_ROTATION.clone(),
                scale: vec3(1.0, 1.0, 1.0),
            },
            MeshID(mesh_id),
            Car {
                velocity: vec3(0.0, 0.0, 0.0),
                turn_speed: 5.0,
                speed: 10.0,
            },
        ));
        self.car_entity = Some(entity);
    }

    pub fn get_draw_calls(&self) -> HashMap<usize, Vec<DrawInstance>> {
        let mut instanced_draw_calls: HashMap<usize, Vec<DrawInstance>> = HashMap::new();

        for (_, (transform, mesh_id)) in self.world.query::<(&Transform, &MeshID)>().iter() {
            let translation = nalgebra_glm::translation(&transform.position);
            let scale = nalgebra_glm::scaling(&transform.scale);

            let model_matrix = translation * transform.rotation * scale;
            let normal_matrix = nalgebra_glm::inverse_transpose(model_matrix);

            let draw_instances = instanced_draw_calls.entry(mesh_id.0).or_default();
            draw_instances.push(DrawInstance::new(model_matrix, normal_matrix));
        }

        instanced_draw_calls
    }

    pub fn tick(&mut self, delta: f32) {
        car_system(&mut self.world, &self.input_manager, delta);
    }
}
