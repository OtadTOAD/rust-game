use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::memory::allocator::StandardMemoryAllocator;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OctreeMetadata {
    pub octree_size: u32,
    pub node_count: u32,
    pub max_depth: u32,
    pub _padding: u32,
}

unsafe impl bytemuck::Pod for OctreeMetadata {}
unsafe impl bytemuck::Zeroable for OctreeMetadata {}

impl OctreeMetadata {
    pub fn new(octree_size: u32, node_count: usize, max_depth: u8) -> Self {
        Self {
            octree_size,
            node_count: node_count as u32,
            max_depth: max_depth as u32,
            _padding: 0,
        }
    }

    pub fn create_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<OctreeMetadata>> {
        CpuAccessibleBuffer::from_data(
            allocator,
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            *self,
        )
        .expect("Failed to create metadata buffer")
    }
}
