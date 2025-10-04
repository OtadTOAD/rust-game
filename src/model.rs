use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{TMat4, TVec3, identity, inverse_transpose, rotate_normalized_axis, translate};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct NormalVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

/// A vertex type intended to be used to provide dummy rendering
/// data for rendering passes that do not require geometry data.
/// This is due to a quirk of the Vulkan API in that *all*
/// render passes require some sort of input.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct DummyVertex {
    /// A regular position vector with the z-value shaved off for space.
    /// This assumes the shaders will take a `vec2` and transform it as
    /// needed.
    pub position: [f32; 2],
}

impl DummyVertex {
    /// This is intended to compliment the use of this data type for passing to
    /// deferred rendering passes that do not actually require geometry input.
    /// This list will draw a square across the entire rendering area. This will
    /// cause the fragment shaders to execute on all pixels in the rendering
    /// area.
    pub fn list() -> [DummyVertex; 6] {
        [
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [-1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, -1.0],
            },
        ]
    }
}

pub struct Model {
    data: Vec<NormalVertex>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    model: TMat4<f32>,
    normals: TMat4<f32>,

    // Incase we want to do multiple rotation translations without updating model each time.
    // (Only updating when we request it)
    required_update: bool,
}

pub struct ModelBuilder {
    file_name: String,
    custom_color: [f32; 3],
}

impl ModelBuilder {
    fn new(file: String) -> ModelBuilder {
        ModelBuilder {
            file_name: file,
            custom_color: [1.0, 0.35, 0.137],
        }
    }

    pub fn build(self) -> Model {
        let (gltf, buffers, _) = gltf::import(self.file_name).expect("Failed to open glTF");

        let mut vertices: Vec<NormalVertex> = Vec::new();

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .expect("Mesh has no POSITION attribute")
                    .collect();

                let normals: Vec<[f32; 3]> = if let Some(iter) = reader.read_normals() {
                    iter.collect()
                } else {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                };

                let colors: Vec<[f32; 3]> = if let Some(iter) = reader.read_colors(0) {
                    iter.into_rgb_f32().collect()
                } else {
                    vec![self.custom_color; positions.len()]
                };

                let indices: Option<Vec<u32>> =
                    reader.read_indices().map(|i| i.into_u32().collect());

                if let Some(indices) = indices {
                    for tri in indices.chunks(3) {
                        // Currently reversing order because I am flattening this and it doesn't work otherwise
                        // In future have vertex/index buffers and use that instead
                        vertices.push(NormalVertex {
                            position: positions[tri[2] as usize],
                            normal: normals[tri[2] as usize],
                            color: colors[tri[2] as usize],
                        });
                        vertices.push(NormalVertex {
                            position: positions[tri[1] as usize],
                            normal: normals[tri[1] as usize],
                            color: colors[tri[1] as usize],
                        });
                        vertices.push(NormalVertex {
                            position: positions[tri[0] as usize],
                            normal: normals[tri[0] as usize],
                            color: colors[tri[0] as usize],
                        });
                    }
                } else {
                    for i in 0..positions.len() {
                        vertices.push(NormalVertex {
                            position: positions[i],
                            normal: normals[i],
                            color: colors[i],
                        });
                    }
                }
            }
        }

        Model {
            data: vertices,
            translation: nalgebra_glm::identity(),
            rotation: nalgebra_glm::identity(),
            model: nalgebra_glm::identity(),
            normals: nalgebra_glm::identity(),
            required_update: false,
        }
    }

    pub fn color(mut self, new_color: [f32; 3]) -> ModelBuilder {
        self.custom_color = new_color;
        self
    }

    pub fn file(mut self, file: String) -> ModelBuilder {
        self.file_name = file;
        self
    }
}

impl Model {
    pub fn new(file_name: &str) -> ModelBuilder {
        ModelBuilder::new(file_name.into())
    }

    pub fn data(&self) -> Vec<NormalVertex> {
        self.data.clone()
    }

    pub fn model_matrix(&mut self) -> TMat4<f32> {
        if self.required_update {
            self.model = self.translation * self.rotation;
            self.required_update = false;
        }
        self.model
    }

    pub fn model_matrices(&mut self) -> (TMat4<f32>, TMat4<f32>) {
        if self.required_update {
            self.model = self.translation * self.rotation;
            self.normals = inverse_transpose(self.model);
            self.required_update = false;
        }
        (self.model, self.normals)
    }

    pub fn rotate(&mut self, radians: f32, v: TVec3<f32>) {
        self.rotation = rotate_normalized_axis(&self.rotation, radians, &v);
        self.required_update = true;
    }

    pub fn translate(&mut self, v: TVec3<f32>) {
        self.translation = translate(&self.translation, &v);
        self.required_update = true;
    }

    pub fn zero_rotation(&mut self) {
        self.rotation = identity();
        self.required_update = true;
    }
}
