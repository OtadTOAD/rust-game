use bytemuck::{Pod, Zeroable};

// A vertex type intended to be used to provide dummy rendering
// data for rendering passes that do not require geometry data.
// This is due to a quirk of the Vulkan API in that *all*
// render passes require some sort of input.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct DummyVertex {
    // A regular position vector with the z-value shaved off for space.
    // This assumes the shaders will take a `vec2` and transform it as
    // needed.
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

// A simple vertex type for rendering boxes. Since all models will be treated as
// Bounding boxes for rendering purposes, this vertex type is sufficient.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct BoxVertex {
    pub in_position: [f32; 3],
}

impl BoxVertex {
    pub fn list() -> [BoxVertex; 36] {
        // Generate proper cube vertices with 6 faces, 2 triangles per face
        // Each triangle has 3 vertices
        [
            // Front face (z = 0.5)
            BoxVertex {
                in_position: [-0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, 0.5],
            },
            // Back face (z = -0.5)
            BoxVertex {
                in_position: [0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, -0.5],
            },
            // Left face (x = -0.5)
            BoxVertex {
                in_position: [-0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, -0.5],
            },
            // Right face (x = 0.5)
            BoxVertex {
                in_position: [0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, 0.5],
            },
            // Top face (y = 0.5)
            BoxVertex {
                in_position: [-0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, 0.5],
            },
            BoxVertex {
                in_position: [0.5, 0.5, -0.5],
            },
            BoxVertex {
                in_position: [-0.5, 0.5, -0.5],
            },
            // Bottom face (y = -0.5)
            BoxVertex {
                in_position: [-0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, -0.5],
            },
            BoxVertex {
                in_position: [0.5, -0.5, 0.5],
            },
            BoxVertex {
                in_position: [-0.5, -0.5, 0.5],
            },
        ]
    }
}
