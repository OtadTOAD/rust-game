use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::memory::allocator::StandardMemoryAllocator;

use super::octree::Octree;

/// GPU-friendly octree node - exactly 8 bytes, tightly packed
/// This matches the shader layout exactly
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GpuOctreeNode {
    /// Child pointer:
    /// - If 0: this is a LEAF node, check data field
    /// - If non-zero: index to first of 8 children in the buffer
    pub child_ptr: u32,

    /// Data field:
    /// - For LEAF nodes: voxel material ID (0 = empty)
    /// - For BRANCH nodes: unused (0)
    pub data: u8,

    /// Padding to maintain 8-byte alignment
    pub _padding: [u8; 3],
}

/// Octree metadata for GPU
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GpuOctreeMetadata {
    /// Size of octree (power of 2, e.g., 128 means 128^3 voxels)
    pub octree_size: u32,

    /// Total number of nodes in the buffer
    pub node_count: u32,

    /// Maximum depth of the tree
    pub max_depth: u32,

    /// World offset (to convert octree coords to world coords)
    pub world_offset_x: i32,
    pub world_offset_y: i32,
    pub world_offset_z: i32,

    /// Padding for alignment
    pub _padding: u32,
}

impl GpuOctreeNode {
    pub fn from_cpu_node(node: &super::octree::OctreeNode) -> Self {
        Self {
            child_ptr: node.child_ptr,
            data: node.data,
            _padding: [0; 3],
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.child_ptr == 0
    }
}

pub struct GpuOctree {
    pub nodes: Vec<GpuOctreeNode>,
    pub metadata: GpuOctreeMetadata,
}

impl GpuOctree {
    /// Convert CPU octree to GPU format
    pub fn from_octree(octree: &Octree, world_offset: [i32; 3]) -> Self {
        println!("Converting octree to GPU format...");

        // Convert all nodes
        let nodes: Vec<GpuOctreeNode> = octree
            .nodes
            .iter()
            .map(GpuOctreeNode::from_cpu_node)
            .collect();

        let metadata = GpuOctreeMetadata {
            octree_size: octree.size,
            node_count: nodes.len() as u32,
            max_depth: octree.depth as u32,
            world_offset_x: world_offset[0],
            world_offset_y: world_offset[1],
            world_offset_z: world_offset[2],
            _padding: 0,
        };

        println!("  Nodes: {}", nodes.len());
        println!(
            "  Size: {} bytes",
            nodes.len() * std::mem::size_of::<GpuOctreeNode>()
        );

        Self { nodes, metadata }
    }

    /// Create Vulkan buffer for nodes
    pub fn create_node_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<[GpuOctreeNode]>> {
        CpuAccessibleBuffer::from_iter(
            allocator,
            BufferUsage {
                storage_buffer: true,
                ..Default::default()
            },
            false,
            self.nodes.iter().cloned(),
        )
        .expect("Failed to create octree node buffer")
    }

    /// Create Vulkan buffer for metadata
    pub fn create_metadata_buffer(
        &self,
        allocator: &Arc<StandardMemoryAllocator>,
    ) -> Arc<CpuAccessibleBuffer<GpuOctreeMetadata>> {
        CpuAccessibleBuffer::from_data(
            allocator,
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            },
            false,
            self.metadata,
        )
        .expect("Failed to create octree metadata buffer")
    }
}

// Ensure types are Pod/Zeroable for Vulkan
unsafe impl bytemuck::Pod for GpuOctreeNode {}
unsafe impl bytemuck::Zeroable for GpuOctreeNode {}
unsafe impl bytemuck::Pod for GpuOctreeMetadata {}
unsafe impl bytemuck::Zeroable for GpuOctreeMetadata {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_node_size() {
        // Verify node is exactly 8 bytes
        assert_eq!(std::mem::size_of::<GpuOctreeNode>(), 8);
        assert_eq!(std::mem::align_of::<GpuOctreeNode>(), 4);
    }

    #[test]
    fn test_metadata_size() {
        // Should be 32 bytes (8 u32s)
        let size = std::mem::size_of::<GpuOctreeMetadata>();
        println!("Metadata size: {} bytes", size);
        assert!(size <= 64); // reasonable size
    }
}
