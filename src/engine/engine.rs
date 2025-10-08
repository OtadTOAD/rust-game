use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use hecs::{Entity, World};
use nalgebra_glm::{TMat4, Vec3, identity, pi, rotate_normalized_axis, rotation, vec3};
use once_cell::sync::Lazy;

use crate::engine::{
    DrawInstance, InputManager, Mesh, Skybox,
    ecs::{Car, MaterialID, MeshID, Transform, car_system},
    material::Material,
};

static DEFAULT_ROTATION: Lazy<TMat4<f32>> = Lazy::new(|| {
    let default = identity();
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 0.0, 1.0));
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 1.0, 0.0));

    default
});

pub struct Camera {
    pub view: TMat4<f32>,
    pub camera_pos: Vec3,
    pub requires_update: bool,
}

pub struct Engine {
    pub input_manager: InputManager,
    pub meshes: HashMap<usize, Arc<Mesh>>,
    pub materials: HashMap<usize, Arc<RwLock<Material>>>,
    pub world: World,
    pub skybox: Skybox,

    pub camera: Camera,
    car_entity: Option<Entity>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            input_manager: InputManager::new(),
            materials: HashMap::new(),
            meshes: HashMap::new(),

            world: World::new(),

            skybox: Skybox::new("assets/HDR/forest.exr"),
            camera: Camera {
                view: identity(),
                camera_pos: vec3(0.0, 0.0, 0.0),
                requires_update: false,
            },
            car_entity: None,
        }
    }

    pub fn init(&mut self) {
        self.load_material(0, "default");
        self.load_material(1, "material_cube");
        self.load_mesh(0, "Material_Test");

        let entity = self.spawn_instance(0, 1, vec3(0.0, -1.5, -3.0));
        if let Ok(transform) = self.world.query_one_mut::<&mut Transform>(entity) {
            let rotation = rotate_normalized_axis(
                &transform.rotation,
                pi::<f32>() * 0.5,
                &vec3(0.0, 1.0, 0.0),
            );
            transform.rotation = rotation;
        }

        /*
        let car_mesh_id = self.load_mesh("assets/meshes/Cart.glb");
        self.spawn_car(car_mesh_id, vec3(0.0, 0.0, -6.0));
        let goal_mesh_id = self.load_mesh("assets/meshes/Goal.glb");
        self.spawn_instance(goal_mesh_id, vec3(0.0, 0.0, -15.0));
        let map_mesh_id = self.load_mesh("assets/meshes/Map.glb");
        self.spawn_instance(map_mesh_id, vec3(0.0, 0.0, 0.0));*/
    }

    pub fn load_material(&mut self, material_id: usize, file_name: &str) {
        let material = Arc::new(RwLock::new(Material::new(file_name)));
        self.materials.insert(material_id.into(), material.clone());
    }

    pub fn load_mesh(&mut self, mesh_id: usize, file_path: &str) {
        let mesh = Mesh::new(file_path);
        self.meshes.insert(mesh_id, Arc::new(mesh));
    }

    pub fn spawn_instance(&mut self, mesh_id: usize, material_id: usize, pos: Vec3) -> Entity {
        self.world.spawn((
            Transform {
                position: pos,
                rotation: DEFAULT_ROTATION.clone(),
                scale: vec3(1.0, 1.0, 1.0),
            },
            MaterialID(material_id),
            MeshID(mesh_id),
        ))
    }

    #[allow(dead_code)]
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
                turn_speed: 1.5,
                speed: 20.0,
            },
        ));
        self.car_entity = Some(entity);
    }

    pub fn get_draw_calls(&self) -> HashMap<(usize, usize), Vec<DrawInstance>> {
        let mut instanced_draw_calls: HashMap<(usize, usize), Vec<DrawInstance>> = HashMap::new();

        for (_, (transform, mesh_id, material_id)) in self
            .world
            .query::<(&Transform, &MeshID, &MaterialID)>()
            .iter()
        {
            let translation = nalgebra_glm::translation(&transform.position);
            let scale = nalgebra_glm::scaling(&transform.scale);

            let model_matrix = translation * transform.rotation * scale;
            let normal_matrix = nalgebra_glm::inverse_transpose(model_matrix);

            let draw_instances = instanced_draw_calls
                .entry((mesh_id.0, material_id.0))
                .or_default();
            draw_instances.push(DrawInstance::new(model_matrix, normal_matrix));
        }

        instanced_draw_calls
    }

    pub fn tick(&mut self, delta: f32) {
        car_system(&mut self.world, &self.input_manager, delta);

        for (_entity, transform) in self.world.query_mut::<&mut Transform>() {
            let mut test = rotation(delta, &vec3(0.0, 1.0, 0.0));
            test *= rotation(delta, &vec3(1.0, 0.0, 0.0));
            transform.rotation = test * transform.rotation;
        }

        if let Some(car_entity) = self.car_entity {
            if let Ok(query) = self
                .world
                .query_one_mut::<(&mut Car, &mut Transform)>(car_entity)
            {
                let (_car, transform) = query;

                let offset_back = -transform.rotation.column(2).xyz() * -5.0;
                let offset_up = vec3(0.0, -3.0, 0.0);
                let desired_pos = transform.position + offset_back + offset_up;

                let lerp_factor = 5.0 * delta;
                let camera_pos =
                    self.camera.camera_pos + (desired_pos - self.camera.camera_pos) * lerp_factor;

                let forward_dir = -transform.rotation.column(2).xyz();
                let look_ahead_distance = 5.0; // how far in front to look
                let target_pos = transform.position + forward_dir * look_ahead_distance;

                let view_matrix: TMat4<f32> =
                    nalgebra_glm::look_at(&camera_pos, &target_pos, &vec3(0.0, 1.0, 0.0));

                self.camera.camera_pos = camera_pos;
                self.camera.view = view_matrix;
                self.camera.requires_update = true;
            }
        }
    }
}
