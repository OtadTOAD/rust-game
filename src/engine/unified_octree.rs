use nalgebra_glm::{IVec3, vec3};
use std::collections::HashSet;

pub const MAX_OCTREE_DEPTH: u8 = 10;
pub const CHUNK_SIZE: i32 = 16;
pub type VoxelID = u8;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OctreeNode {
    pub child_ptr: u32,
    pub data: u8,
    pub depth: u8,
    _padding: [u8; 2],
}

impl OctreeNode {
    #[inline]
    pub fn new_leaf(voxel_id: VoxelID, depth: u8) -> Self {
        Self {
            child_ptr: 0,
            data: voxel_id,
            depth,
            _padding: [0; 2],
        }
    }

    #[inline]
    pub fn new_branch(child_ptr: u32, depth: u8) -> Self {
        Self {
            child_ptr,
            data: 0,
            depth,
            _padding: [0; 2],
        }
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.child_ptr == 0
    }
}

unsafe impl bytemuck::Pod for OctreeNode {}
unsafe impl bytemuck::Zeroable for OctreeNode {}

pub struct UnifiedOctree {
    pub nodes: Vec<OctreeNode>,
    pub size: u32,
    pub depth: u8,

    free_list: Vec<u32>,

    dirty_nodes: HashSet<usize>,
    structure_changed: bool,

    pub total_updates: u64,
}

impl UnifiedOctree {
    pub fn new(size: u32) -> Self {
        assert!(size.is_power_of_two(), "Size must be power of 2");
        let depth = size.trailing_zeros() as u8;
        assert!(depth <= MAX_OCTREE_DEPTH);

        Self {
            nodes: vec![OctreeNode::new_leaf(0, 0)],
            size,
            depth,
            free_list: Vec::new(),
            dirty_nodes: HashSet::new(),
            structure_changed: false,
            total_updates: 0,
        }
    }

    pub fn from_generator<F>(size: u32, generator: F) -> Self
    where
        F: Fn(i32, i32, i32) -> VoxelID,
    {
        let mut octree = Self::new(size);
        octree.build_from_function(0, vec3(0, 0, 0), size as i32, 0, &generator);
        octree.structure_changed = true;
        octree
    }

    fn build_from_function<F>(
        &mut self,
        node_idx: usize,
        node_min: IVec3,
        node_size: i32,
        current_depth: u8,
        generator: &F,
    ) where
        F: Fn(i32, i32, i32) -> VoxelID,
    {
        if current_depth >= self.depth || node_size == 1 {
            self.nodes[node_idx].data = generator(node_min.x, node_min.y, node_min.z);
            return;
        }

        let first = generator(node_min.x, node_min.y, node_min.z);
        let sample_step = if node_size <= 8 { 1 } else { node_size / 8 };

        let mut is_uniform = true;
        'check: for z in (0..node_size).step_by(sample_step as usize) {
            for y in (0..node_size).step_by(sample_step as usize) {
                for x in (0..node_size).step_by(sample_step as usize) {
                    if generator(node_min.x + x, node_min.y + y, node_min.z + z) != first {
                        is_uniform = false;
                        break 'check;
                    }
                }
            }
        }

        if is_uniform {
            self.nodes[node_idx].data = first;
            return;
        }

        let half_size = node_size / 2;
        let child_ptr = self.nodes.len() as u32;

        for _ in 0..8 {
            self.nodes.push(OctreeNode::new_leaf(0, current_depth + 1));
        }

        self.nodes[node_idx] = OctreeNode::new_branch(child_ptr, current_depth);

        for i in 0..8 {
            let offset = vec3(
                if i & 1 != 0 { half_size } else { 0 },
                if i & 2 != 0 { half_size } else { 0 },
                if i & 4 != 0 { half_size } else { 0 },
            );
            self.build_from_function(
                (child_ptr as usize) + i,
                node_min + offset,
                half_size,
                current_depth + 1,
                generator,
            );
        }

        self.try_merge_children(node_idx);
    }

    #[inline]
    pub fn set_voxel(&mut self, pos: IVec3, voxel_id: VoxelID) {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return;
        }

        self.total_updates += 1;
        self.set_voxel_recursive(0, pos, voxel_id, vec3(0, 0, 0), size, 0);
    }

    fn set_voxel_recursive(
        &mut self,
        node_idx: usize,
        pos: IVec3,
        voxel_id: VoxelID,
        node_min: IVec3,
        node_size: i32,
        current_depth: u8,
    ) -> bool {
        self.dirty_nodes.insert(node_idx);

        if current_depth >= self.depth || node_size == 1 {
            if self.nodes[node_idx].data != voxel_id {
                self.nodes[node_idx].data = voxel_id;
                return true;
            }
            return false;
        }

        let half_size = node_size / 2;
        let mid = node_min + vec3(half_size, half_size, half_size);
        let child_idx = Self::get_child_index(pos, mid);

        if self.nodes[node_idx].is_leaf() {
            let current_voxel = self.nodes[node_idx].data;
            if current_voxel == voxel_id {
                return false;
            }

            self.structure_changed = true;
            let child_ptr = self.allocate_children(current_depth + 1);

            for i in 0..8 {
                self.nodes[(child_ptr as usize) + i].data = current_voxel;
            }

            self.nodes[node_idx] = OctreeNode::new_branch(child_ptr, current_depth);
        }

        let child_min = vec3(
            if pos.x >= mid.x { mid.x } else { node_min.x },
            if pos.y >= mid.y { mid.y } else { node_min.y },
            if pos.z >= mid.z { mid.z } else { node_min.z },
        );

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        let changed = self.set_voxel_recursive(
            child_ptr + child_idx,
            pos,
            voxel_id,
            child_min,
            half_size,
            current_depth + 1,
        );

        if changed {
            self.try_merge_children(node_idx);
        }

        changed
    }

    pub fn set_region<F>(&mut self, min: IVec3, max: IVec3, mut generator: F)
    where
        F: FnMut(i32, i32, i32) -> VoxelID,
    {
        for z in min.z..=max.z {
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    let voxel = generator(x, y, z);
                    if voxel != 0 {
                        self.set_voxel(vec3(x, y, z), voxel);
                    }
                }
            }
        }
    }

    #[inline]
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

    #[inline]
    fn get_child_index(pos: IVec3, mid: IVec3) -> usize {
        ((pos.x >= mid.x) as usize)
            | (((pos.y >= mid.y) as usize) << 1)
            | (((pos.z >= mid.z) as usize) << 2)
    }

    fn allocate_children(&mut self, depth: u8) -> u32 {
        if self.free_list.len() >= 8 {
            let ptr = self.free_list.pop().unwrap();
            for i in 0..8 {
                self.nodes[(ptr as usize) + i] = OctreeNode::new_leaf(0, depth);
            }
            return ptr;
        }

        let ptr = self.nodes.len() as u32;
        self.nodes
            .extend((0..8).map(|_| OctreeNode::new_leaf(0, depth)));
        ptr
    }

    fn free_children(&mut self, child_ptr: u32) {
        self.free_list.push(child_ptr);
    }

    fn try_merge_children(&mut self, node_idx: usize) {
        if self.nodes[node_idx].is_leaf() {
            return;
        }

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        let first = &self.nodes[child_ptr];

        if !first.is_leaf() {
            return;
        }

        let first_value = first.data;
        for i in 1..8 {
            let child = &self.nodes[child_ptr + i];
            if !child.is_leaf() || child.data != first_value {
                return;
            }
        }

        self.structure_changed = true;
        let old_ptr = self.nodes[node_idx].child_ptr;
        self.nodes[node_idx] = OctreeNode::new_leaf(first_value, self.nodes[node_idx].depth);
        self.free_children(old_ptr);
    }

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
        self.dirty_nodes.clear();
        self.structure_changed = true;
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

    pub fn needs_gpu_upload(&self) -> bool {
        !self.dirty_nodes.is_empty() || self.structure_changed
    }

    pub fn needs_full_upload(&self) -> bool {
        self.structure_changed
    }

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

    pub fn clear_dirty_state(&mut self) {
        self.dirty_nodes.clear();
        self.structure_changed = false;
    }

    pub fn get_stats(&self) -> OctreeStats {
        let mut stats = OctreeStats {
            total_nodes: self.nodes.len(),
            free_nodes: self.free_list.len() * 8,
            total_updates: self.total_updates,
            ..Default::default()
        };

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
            stats.max_depth = stats.max_depth.max(node.depth);
        }

        stats
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
    pub free_nodes: usize,
    pub total_updates: u64,
}

impl OctreeStats {
    pub fn memory_usage(&self) -> usize {
        self.total_nodes * std::mem::size_of::<OctreeNode>()
    }

    pub fn print(&self) {
        let mem_mb = self.memory_usage() as f32 / (1024.0 * 1024.0);
        let fragmentation = (self.free_nodes as f32 / self.total_nodes.max(1) as f32) * 100.0;

        println!("\n╔═══════════════════════════════════════╗");
        println!("║       Octree Statistics               ║");
        println!("╠═══════════════════════════════════════╣");
        println!("║ Total nodes:    {:>20} ║", self.total_nodes);
        println!("║ Branch nodes:   {:>20} ║", self.branch_nodes);
        println!("║ Leaf nodes:     {:>20} ║", self.leaf_nodes);
        println!("║ Filled leaves:  {:>20} ║", self.filled_leaf_nodes);
        println!("║ Free nodes:     {:>20} ║", self.free_nodes);
        println!("║ Memory:         {:>17.2} MB ║", mem_mb);
        println!("║ Fragmentation:  {:>17.1}% ║", fragmentation);
        println!("╚═══════════════════════════════════════╝\n");
    }
}
