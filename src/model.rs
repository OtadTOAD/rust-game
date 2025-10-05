#![allow(dead_code)]

use std::{io::Cursor, sync::Arc};

use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{
    TMat4, TVec3, identity, inverse_transpose, pi, rotate_normalized_axis, scale, translate, vec3,
};
use once_cell::sync::Lazy;
use vulkano::image::{ImageDimensions, ImmutableImage, view::ImageView};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct NormalVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct ColoredVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Clone)]
pub struct Texture {
    pub data: Vec<u8>,
    pub dimensions: ImageDimensions,
}

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<NormalVertex>,
    pub indices: Vec<u32>,
}

pub struct Model {
    mesh: Mesh,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    scale_factor: f32,
    model: TMat4<f32>,
    normals: TMat4<f32>,
    specular_intensity: f32,
    shininess: f32,

    pub texture_instance: Option<Arc<ImageView<ImmutableImage>>>,
    texture: Texture,

    // Incase we want to do multiple rotation translations without updating model each time.
    // (Only updating when we request it)
    required_update: bool,
}

pub struct ModelBuilder {
    file_name: String,
    custom_color: [f32; 3],
    scale_factor: f32,
    specular_intensity: f32,
    shininess: f32,
    texture: String,
}

static DEFAULT_ROTATION: Lazy<TMat4<f32>> = Lazy::new(|| {
    let default = identity();
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 0.0, 1.0));
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 1.0, 0.0));

    default
});

impl ModelBuilder {
    fn new(file: String, texture: String) -> ModelBuilder {
        ModelBuilder {
            file_name: file,
            custom_color: [1.0, 0.35, 0.137],
            scale_factor: 1.0,
            specular_intensity: 0.5,
            shininess: 32.0,
            texture,
        }
    }

    pub fn build(self) -> Model {
        let (gltf, buffers, _) = gltf::import(self.file_name).expect("Failed to open glTF");

        let mut vertices: Vec<NormalVertex> = Vec::new();
        let mut indices = Vec::new();

        for mesh in gltf.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Keep in mind because glTF uses diff coord system than Vulkan, we need to flip Z axis
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .expect("Mesh has no POSITION attribute")
                    .map(|[x, y, z]| [x, y, -z])
                    .collect();
                let normals: Vec<[f32; 3]> = if let Some(iter) = reader.read_normals() {
                    iter.map(|[x, y, z]| [x, y, -z]).collect()
                } else {
                    vec![[0.0, 1.0, 0.0]; positions.len()]
                };

                let colors: Vec<[f32; 3]> = reader
                    .read_colors(0)
                    .map(|c| c.into_rgb_f32().collect())
                    .unwrap_or_else(|| vec![self.custom_color; positions.len()]);

                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|t| t.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                let start_index = vertices.len() as u32;
                for i in 0..positions.len() {
                    vertices.push(NormalVertex {
                        position: positions[i],
                        normal: normals[i],
                        color: colors[i],
                        uv: uvs[i],
                    });
                }

                // Add indices
                if let Some(read_indices) = reader.read_indices() {
                    indices.extend(read_indices.into_u32().map(|i| i + start_index));
                } else {
                    indices.extend((0..positions.len() as u32).map(|i| i + start_index));
                }
            }
        }

        let (image_data, image_dimensions) = {
            let png_bytes = std::fs::read(self.texture).unwrap();
            let cursor = Cursor::new(png_bytes);
            let decoder = png::Decoder::new(cursor);
            let mut reader = decoder.read_info().unwrap();
            let info = reader.info();
            let image_dimensions = ImageDimensions::Dim2d {
                width: info.width,
                height: info.height,
                array_layers: 1,
            };
            let mut image_data = Vec::new();
            let depth: u32 = match info.bit_depth {
                png::BitDepth::One => 1,
                png::BitDepth::Two => 2,
                png::BitDepth::Four => 4,
                png::BitDepth::Eight => 8,
                png::BitDepth::Sixteen => 16,
            };
            image_data.resize((info.width * info.height * depth) as usize, 0);
            reader.next_frame(&mut image_data).unwrap();
            (image_data, image_dimensions)
        };

        let texture: Option<Arc<ImageView<_>>> = None;

        Model {
            mesh: Mesh {
                vertices: vertices,
                indices: indices,
            },
            scale_factor: self.scale_factor,
            specular_intensity: self.specular_intensity,
            shininess: self.shininess,

            translation: nalgebra_glm::identity(),
            rotation: DEFAULT_ROTATION.clone(),
            model: nalgebra_glm::identity(),
            normals: nalgebra_glm::identity(),

            texture_instance: texture,
            texture: Texture {
                data: image_data,
                dimensions: image_dimensions,
            },

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

    pub fn specular(mut self, specular_intensity: f32, shininess: f32) -> ModelBuilder {
        self.specular_intensity = specular_intensity;
        self.shininess = shininess;
        self
    }

    pub fn uniform_scale_factor(mut self, scale: f32) -> ModelBuilder {
        self.scale_factor = scale;
        self
    }
}

impl Model {
    pub fn new(file_name: &str, texture_name: &str) -> ModelBuilder {
        ModelBuilder::new(file_name.into(), texture_name.into())
    }

    pub fn mesh(&self) -> Mesh {
        self.mesh.clone()
    }

    pub fn texture_data(&self) -> Texture {
        self.texture.clone()
    }

    pub fn color_data(&self) -> (Vec<ColoredVertex>, Vec<u32>) {
        let mut vertices: Vec<ColoredVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        indices.extend(&self.mesh.indices);
        for v in &self.mesh.vertices {
            vertices.push(ColoredVertex {
                position: v.position,
                color: v.color,
            });
        }

        (vertices, indices)
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
            self.model = scale(
                &self.model,
                &vec3(self.scale_factor, self.scale_factor, self.scale_factor),
            );
            self.normals = inverse_transpose(self.model);
            self.required_update = false;
        }
        (self.model, self.normals)
    }

    pub fn rotate(&mut self, radians: f32, v: TVec3<f32>) {
        self.rotation = rotate_normalized_axis(&self.rotation, radians, &v);
        self.required_update = true;
    }

    pub fn specular(&self) -> (f32, f32) {
        (self.specular_intensity, self.shininess)
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
