use nalgebra_glm::{IVec3, vec3};
use std::collections::HashSet;

pub const MAX_OCTREE_DEPTH: u8 = 10;
pub type VoxelID = u8;

#[derive(Debug, Clone, Copy)]
pub struct OctreeNode {
    pub child_ptr: u32,
    pub data: u8,
    _padding: [u8; 3],
}

impl OctreeNode {
    #[inline]
    pub fn new_leaf(voxel_id: VoxelID) -> Self {
        Self {
            child_ptr: 0,
            data: voxel_id,
            _padding: [0; 3],
        }
    }

    #[inline]
    pub fn new_branch(child_ptr: u32) -> Self {
        Self {
            child_ptr,
            data: 0,
            _padding: [0; 3],
        }
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.child_ptr == 0
    }
}

pub struct OctreeWorld {
    pub nodes: Vec<OctreeNode>,
    pub size: u32,
    pub depth: u8,

    // Simplified memory management
    free_list: Vec<u32>,

    // GPU sync
    dirty_nodes: HashSet<usize>,
    needs_full_rebuild: bool,
}

impl OctreeWorld {
    pub fn new(size: u32) -> Self {
        assert!(size.is_power_of_two());
        let depth = size.trailing_zeros() as u8;

        Self {
            nodes: vec![OctreeNode::new_leaf(0)],
            size,
            depth,
            free_list: Vec::new(),
            dirty_nodes: HashSet::new(),
            needs_full_rebuild: false,
        }
    }

    /// Allocate 8 children
    fn allocate_children(&mut self) -> u32 {
        if self.free_list.len() >= 8 {
            let ptr = self.free_list.pop().unwrap();
            for i in 0..8 {
                self.nodes[(ptr as usize) + i] = OctreeNode::new_leaf(0);
            }
            return ptr;
        }

        let ptr = self.nodes.len() as u32;
        self.nodes.extend((0..8).map(|_| OctreeNode::new_leaf(0)));
        ptr
    }

    #[inline]
    fn free_children(&mut self, ptr: u32) {
        self.free_list.push(ptr);
    }

    #[inline]
    fn get_child_index(pos: IVec3, mid: IVec3) -> usize {
        ((pos.x >= mid.x) as usize)
            | (((pos.y >= mid.y) as usize) << 1)
            | (((pos.z >= mid.z) as usize) << 2)
    }

    /// Set a voxel - this is your primary interface
    pub fn set_voxel(&mut self, pos: IVec3, voxel_id: VoxelID) {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return;
        }

        self.set_voxel_recursive(0, pos, voxel_id, vec3(0, 0, 0), size);
    }

    fn set_voxel_recursive(
        &mut self,
        node_idx: usize,
        pos: IVec3,
        voxel_id: VoxelID,
        node_min: IVec3,
        node_size: i32,
    ) {
        self.dirty_nodes.insert(node_idx);

        // Leaf node
        if node_size == 1 {
            if self.nodes[node_idx].data != voxel_id {
                self.nodes[node_idx].data = voxel_id;
            }
            return;
        }

        let half_size = node_size / 2;
        let mid = node_min + vec3(half_size, half_size, half_size);
        let child_idx = Self::get_child_index(pos, mid);

        // Split leaf if needed
        if self.nodes[node_idx].is_leaf() {
            let current_voxel = self.nodes[node_idx].data;

            if current_voxel == voxel_id {
                return; // Already correct
            }

            self.needs_full_rebuild = true;
            let child_ptr = self.allocate_children();

            for i in 0..8 {
                self.nodes[(child_ptr as usize) + i].data = current_voxel;
            }

            self.nodes[node_idx] = OctreeNode::new_branch(child_ptr);
        }

        // Recurse to child
        let child_min = vec3(
            if pos.x >= mid.x { mid.x } else { node_min.x },
            if pos.y >= mid.y { mid.y } else { node_min.y },
            if pos.z >= mid.z { mid.z } else { node_min.z },
        );

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        self.set_voxel_recursive(child_ptr + child_idx, pos, voxel_id, child_min, half_size);

        // Try to merge children
        self.try_merge_children(node_idx);
    }

    fn try_merge_children(&mut self, node_idx: usize) {
        if self.nodes[node_idx].is_leaf() {
            return;
        }

        let child_ptr = self.nodes[node_idx].child_ptr as usize;

        // Check if all children are identical leaves
        let first = &self.nodes[child_ptr];
        if !first.is_leaf() {
            return;
        }

        let value = first.data;
        for i in 1..8 {
            let child = &self.nodes[child_ptr + i];
            if !child.is_leaf() || child.data != value {
                return;
            }
        }

        // Merge
        self.needs_full_rebuild = true;
        let old_ptr = self.nodes[node_idx].child_ptr;
        self.nodes[node_idx] = OctreeNode::new_leaf(value);
        self.free_children(old_ptr);
    }

    /// Get a voxel
    pub fn get_voxel(&self, pos: IVec3) -> VoxelID {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return 0;
        }

        self.get_voxel_recursive(0, pos, vec3(0, 0, 0), size)
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

        self.get_voxel_recursive(
            (node.child_ptr as usize) + child_idx,
            pos,
            child_min,
            half_size,
        )
    }

    /// Generate a sphere directly into the octree
    pub fn generate_sphere(&mut self, center: IVec3, radius: f32, voxel_id: VoxelID) {
        let r_sq = radius * radius;
        let min = center - vec3(radius as i32 + 1, radius as i32 + 1, radius as i32 + 1);
        let max = center + vec3(radius as i32 + 1, radius as i32 + 1, radius as i32 + 1);

        for z in min.z..=max.z {
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    let dx = (x - center.x) as f32;
                    let dy = (y - center.y) as f32;
                    let dz = (z - center.z) as f32;

                    if dx * dx + dy * dy + dz * dz < r_sq {
                        self.set_voxel(vec3(x, y, z), voxel_id);
                    }
                }
            }
        }
    }

    /// Clear dirty state after GPU upload
    pub fn clear_gpu_dirty_state(&mut self) {
        self.dirty_nodes.clear();
        self.needs_full_rebuild = false;
    }

    /// Get dirty node ranges for partial GPU updates
    pub fn get_dirty_ranges(&self) -> Vec<(usize, usize)> {
        if self.dirty_nodes.is_empty() {
            return Vec::new();
        }

        let mut sorted: Vec<_> = self.dirty_nodes.iter().copied().collect();
        sorted.sort_unstable();

        let mut ranges = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0] + 1;

        for &idx in &sorted[1..] {
            if idx <= end + 8 {
                end = idx + 1;
            } else {
                ranges.push((start, end));
                start = idx;
                end = idx + 1;
            }
        }
        ranges.push((start, end));

        ranges
    }

    pub fn needs_full_gpu_rebuild(&self) -> bool {
        self.needs_full_rebuild
    }

    /// Compact to remove fragmentation
    pub fn compact(&mut self) {
        let mut reachable = vec![false; self.nodes.len()];
        self.mark_reachable(0, &mut reachable);

        let new_size = reachable.iter().filter(|&&r| r).count();
        if new_size == self.nodes.len() {
            return;
        }

        let mut new_indices = vec![0u32; self.nodes.len()];
        let mut new_idx = 0u32;

        for (old_idx, &is_reachable) in reachable.iter().enumerate() {
            if is_reachable {
                new_indices[old_idx] = new_idx;
                new_idx += 1;
            }
        }

        let mut new_nodes = Vec::with_capacity(new_size);
        for (old_idx, node) in self.nodes.iter().enumerate() {
            if reachable[old_idx] {
                let mut new_node = *node;
                if !new_node.is_leaf() {
                    new_node.child_ptr = new_indices[new_node.child_ptr as usize];
                }
                new_nodes.push(new_node);
            }
        }

        self.nodes = new_nodes;
        self.free_list.clear();
        self.needs_full_rebuild = true;
    }

    fn mark_reachable(&self, node_idx: usize, reachable: &mut [bool]) {
        if node_idx >= self.nodes.len() || reachable[node_idx] {
            return;
        }

        reachable[node_idx] = true;

        if !self.nodes[node_idx].is_leaf() {
            let child_ptr = self.nodes[node_idx].child_ptr as usize;
            for i in 0..8 {
                self.mark_reachable(child_ptr + i, reachable);
            }
        }
    }
}
