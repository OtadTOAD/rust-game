use nalgebra_glm::{IVec3, vec3};

/// Maximum octree depth (adjust based on your world size)
pub const MAX_OCTREE_DEPTH: u8 = 10;

/// Represents a single voxel type/material ID
pub type VoxelID = u8;

/// Compact node representation - only 8 bytes!
#[derive(Debug, Clone, Copy)]
pub struct OctreeNode {
    /// Child pointer: if 0, this is a leaf node
    /// If non-zero, index into nodes array where 8 children start
    pub child_ptr: u32,

    /// For leaf nodes: voxel_id (0 = empty)
    /// For branch nodes: unused
    pub data: u8,

    /// Node depth level
    pub depth: u8,

    /// Padding for alignment
    _padding: [u8; 2],
}

impl OctreeNode {
    pub fn new_leaf(voxel_id: VoxelID, depth: u8) -> Self {
        Self {
            child_ptr: 0,
            data: voxel_id,
            depth,
            _padding: [0; 2],
        }
    }

    pub fn new_branch(child_ptr: u32, depth: u8) -> Self {
        Self {
            child_ptr,
            data: 0,
            depth,
            _padding: [0; 2],
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.child_ptr == 0
    }
}

/// Main octree structure
pub struct Octree {
    /// All nodes stored in a flat array
    /// Children of a node at index i are stored contiguously at children[i]
    pub nodes: Vec<OctreeNode>,

    /// Size of the octree (in voxels per side, must be power of 2)
    pub size: u32,

    /// Actual depth of the tree
    pub depth: u8,
}

impl Octree {
    pub fn new(size: u32) -> Self {
        assert!(size.is_power_of_two(), "Octree size must be power of 2");

        let depth = size.trailing_zeros() as u8;
        assert!(depth <= MAX_OCTREE_DEPTH, "Octree depth exceeds maximum");

        // Start with just a root node (empty)
        let root = OctreeNode::new_leaf(0, 0);

        Self {
            nodes: vec![root],
            size,
            depth,
        }
    }

    fn get_child_index(pos: IVec3, mid: IVec3) -> usize {
        let x = if pos.x >= mid.x { 1 } else { 0 };
        let y = if pos.y >= mid.y { 1 } else { 0 };
        let z = if pos.z >= mid.z { 1 } else { 0 };
        x | (y << 1) | (z << 2)
    }

    /// Set a voxel - OPTIMIZED: only subdivides when necessary
    pub fn set_voxel(&mut self, pos: IVec3, voxel_id: VoxelID) {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return;
        }

        self.set_voxel_recursive(0, pos, voxel_id, vec3(0, 0, 0), self.size as i32, 0);
    }

    fn set_voxel_recursive(
        &mut self,
        node_idx: usize,
        pos: IVec3,
        voxel_id: VoxelID,
        node_min: IVec3,
        node_size: i32,
        current_depth: u8,
    ) {
        // At maximum depth or single voxel, just set it
        if current_depth >= self.depth || node_size == 1 {
            self.nodes[node_idx].data = voxel_id;
            return;
        }

        let half_size = node_size / 2;
        let mid = node_min + vec3(half_size, half_size, half_size);
        let child_idx = Self::get_child_index(pos, mid);

        // If node is a leaf
        if self.nodes[node_idx].is_leaf() {
            let current_voxel = self.nodes[node_idx].data;

            // OPTIMIZATION: Don't subdivide if setting to same value
            if current_voxel == voxel_id {
                return;
            }

            // OPTIMIZATION: Don't subdivide empty leaf to set empty voxel
            if current_voxel == 0 && voxel_id == 0 {
                return;
            }

            // Need to split - allocate 8 children at once
            let child_ptr = self.nodes.len() as u32;

            // Create 8 children with current value
            for _ in 0..8 {
                self.nodes
                    .push(OctreeNode::new_leaf(current_voxel, current_depth + 1));
            }

            // Update parent to point to children
            self.nodes[node_idx].child_ptr = child_ptr;
            self.nodes[node_idx].data = 0;
        }

        // Get child position
        let child_min = vec3(
            if pos.x >= mid.x { mid.x } else { node_min.x },
            if pos.y >= mid.y { mid.y } else { node_min.y },
            if pos.z >= mid.z { mid.z } else { node_min.z },
        );

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        let child_node_idx = child_ptr + child_idx;

        // Recurse into child
        self.set_voxel_recursive(
            child_node_idx,
            pos,
            voxel_id,
            child_min,
            half_size,
            current_depth + 1,
        );

        // OPTIMIZATION: Merge children if they all have the same value
        self.try_merge_children(node_idx);
    }

    /// Try to merge children if they're all leaves with the same value
    fn try_merge_children(&mut self, node_idx: usize) {
        if self.nodes[node_idx].is_leaf() {
            return;
        }

        let child_ptr = self.nodes[node_idx].child_ptr as usize;

        // Check if all 8 children are leaves with same value
        let first_child = &self.nodes[child_ptr];
        if !first_child.is_leaf() {
            return;
        }

        let first_value = first_child.data;

        for i in 1..8 {
            let child = &self.nodes[child_ptr + i];
            if !child.is_leaf() || child.data != first_value {
                return;
            }
        }

        // All children are identical - merge into parent
        self.nodes[node_idx].child_ptr = 0;
        self.nodes[node_idx].data = first_value;

        // Note: We don't actually remove the child nodes from the array
        // to avoid expensive reorganization. They become "garbage" nodes.
        // A full rebuild would clean these up.
    }

    pub fn get_voxel(&self, pos: IVec3) -> VoxelID {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return 0;
        }

        self.get_voxel_recursive(0, pos, vec3(0, 0, 0), self.size as i32)
    }

    fn get_voxel_recursive(
        &self,
        node_idx: usize,
        pos: IVec3,
        node_min: IVec3,
        node_size: i32,
    ) -> VoxelID {
        let node = &self.nodes[node_idx];

        if node.is_leaf() {
            return node.data;
        }

        let half_size = node_size / 2;
        let mid = node_min + vec3(half_size, half_size, half_size);
        let child_idx = Self::get_child_index(pos, mid);

        let child_min = vec3(
            if pos.x >= mid.x { mid.x } else { node_min.x },
            if pos.y >= mid.y { mid.y } else { node_min.y },
            if pos.z >= mid.z { mid.z } else { node_min.z },
        );

        let child_ptr = node.child_ptr as usize;
        self.get_voxel_recursive(child_ptr + child_idx, pos, child_min, half_size)
    }

    pub fn get_stats(&self) -> OctreeStats {
        let mut stats = OctreeStats::default();
        stats.total_nodes = self.nodes.len();

        for node in &self.nodes {
            if node.is_leaf() {
                stats.leaf_nodes += 1;
                if node.data != 0 {
                    stats.filled_leaf_nodes += 1;
                } else {
                    stats.empty_leaf_nodes += 1;
                }
            } else {
                stats.branch_nodes += 1;
            }

            if node.depth > stats.max_depth {
                stats.max_depth = node.depth;
            }
        }

        stats
    }

    /// Compact the octree by removing garbage nodes
    pub fn compact(&mut self) {
        // This is a TODO for later optimization
        // Would rebuild the tree without garbage nodes
    }
}

#[derive(Debug, Default)]
pub struct OctreeStats {
    pub total_nodes: usize,
    pub leaf_nodes: usize,
    pub branch_nodes: usize,
    pub filled_leaf_nodes: usize,
    pub empty_leaf_nodes: usize,
    pub max_depth: u8,
}

impl OctreeStats {
    pub fn memory_usage(&self) -> usize {
        self.total_nodes * std::mem::size_of::<OctreeNode>()
    }

    pub fn compression_ratio(&self, texture_size: u32) -> f32 {
        let texture_voxels = (texture_size * texture_size * texture_size) as usize;
        let texture_bytes = texture_voxels;
        let octree_bytes = self.memory_usage();

        texture_bytes as f32 / octree_bytes as f32
    }

    pub fn garbage_nodes(&self) -> usize {
        // Approximate: nodes that are allocated but not reachable
        // This is a simplified estimate
        0 // TODO: implement proper garbage tracking
    }
}
