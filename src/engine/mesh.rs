#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};
use nalgebra_glm::{TMat4, identity, pi, rotate_normalized_axis, vec3};
use once_cell::sync::Lazy;
use vulkano::image::ImageDimensions;

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

#[derive(Clone)]
pub struct Texture {
    pub data: Vec<u8>,
    pub dimensions: ImageDimensions,
}

#[derive(Clone)]
pub struct Build {
    pub vertices: Vec<NormalVertex>,
    pub indices: Vec<u32>,
}

pub struct Mesh {
    pub build: Build,
    pub tex: Texture,
}

static DEFAULT_COLOR: [f32; 3] = [1.0, 0.35, 0.137];
static DEFAULT_ROTATION: Lazy<TMat4<f32>> = Lazy::new(|| {
    let default = identity();
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 0.0, 1.0));
    let default = rotate_normalized_axis(&default, pi(), &vec3(0.0, 1.0, 0.0));

    default
});

impl Mesh {
    pub fn new(file_path: &str) -> Mesh {
        let (gltf, buffers, images) = gltf::import(file_path).expect("Failed to open glTF");

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
                    .unwrap_or_else(|| vec![DEFAULT_COLOR; positions.len()]);

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

        let (image_data, image_dimensions) = if let Some(first_image) = images.first() {
            let width = first_image.width;
            let height = first_image.height;
            let image_dimensions = ImageDimensions::Dim2d {
                width,
                height,
                array_layers: 1,
            };

            (first_image.pixels.clone(), image_dimensions)
        } else {
            // No texture available - create a default 1x1 white texture
            let default_data = vec![255u8; 4]; // 1x1 RGBA white pixel
            let default_dimensions = ImageDimensions::Dim2d {
                width: 1,
                height: 1,
                array_layers: 1,
            };
            (default_data, default_dimensions)
        };

        Mesh {
            build: Build { vertices, indices },
            tex: Texture {
                data: image_data,
                dimensions: image_dimensions,
            },
        }
    }
}
