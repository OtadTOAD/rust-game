use nalgebra_glm::{IVec3, vec3};
use std::collections::{HashMap, HashSet};

use crate::engine::voxel_chunk::CHUNK_SIZE;

pub const MAX_OCTREE_DEPTH: u8 = 10;
pub type VoxelID = u8;

#[derive(Debug, Clone, Copy)]
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

/// Cached node information for spatial lookups
#[derive(Debug, Clone)]
struct CachedNodeInfo {
    node_idx: usize,
    node_min: IVec3,
    node_size: i32,
    depth: u8,
}

/// Spatial cache for quick chunk->node lookups
struct SpatialCache {
    chunk_to_nodes: HashMap<IVec3, Vec<CachedNodeInfo>>,
    /// Track which chunks have been cached to avoid full rebuilds
    cached_chunks: std::collections::HashSet<IVec3>,
    /// Incremental cache rebuild threshold
    dirty_threshold: usize,
}

impl SpatialCache {
    fn new() -> Self {
        Self {
            chunk_to_nodes: HashMap::new(),
            cached_chunks: std::collections::HashSet::new(),
            dirty_threshold: 10,
        }
    }

    fn clear(&mut self) {
        self.chunk_to_nodes.clear();
        self.cached_chunks.clear();
    }

    fn add_node(&mut self, chunk_pos: IVec3, info: CachedNodeInfo) {
        self.chunk_to_nodes
            .entry(chunk_pos)
            .or_insert_with(Vec::new)
            .push(info);
        self.cached_chunks.insert(chunk_pos);
    }

    fn get_nodes(&self, chunk_pos: &IVec3) -> Option<&Vec<CachedNodeInfo>> {
        self.chunk_to_nodes.get(chunk_pos)
    }

    fn invalidate_chunk(&mut self, chunk_pos: IVec3) {
        self.chunk_to_nodes.remove(&chunk_pos);
        self.cached_chunks.remove(&chunk_pos);
    }

    fn is_cached(&self, chunk_pos: &IVec3) -> bool {
        self.cached_chunks.contains(chunk_pos)
    }
}

pub struct Octree {
    pub nodes: Vec<OctreeNode>,
    pub size: u32,
    pub depth: u8,

    free_list: Vec<u32>,
    spatial_cache: SpatialCache,
    cache_dirty: bool,
    dirty_chunks: std::collections::HashSet<IVec3>,

    pub total_updates: u64,
    pub cache_hits: u64,

    dirty_nodes: HashSet<usize>,
    needs_full_gpu_rebuild: bool,
}

impl Octree {
    pub fn new(size: u32) -> Self {
        assert!(size.is_power_of_two(), "Octree size must be power of 2");
        let depth = size.trailing_zeros() as u8;
        assert!(depth <= MAX_OCTREE_DEPTH, "Octree depth exceeds maximum");

        Self {
            nodes: vec![OctreeNode::new_leaf(0, 0)],
            size,
            depth,
            free_list: Vec::new(),
            spatial_cache: SpatialCache::new(),
            cache_dirty: false,
            dirty_chunks: std::collections::HashSet::new(),
            total_updates: 0,
            cache_hits: 0,
            dirty_nodes: HashSet::new(),
            needs_full_gpu_rebuild: false,
        }
    }

    /// Allocate 8 child nodes, reusing freed nodes when possible
    fn allocate_children(&mut self, depth: u8) -> u32 {
        if self.free_list.len() >= 8 {
            let ptr = self.free_list.pop().unwrap();
            for i in 0..8 {
                let idx = (ptr as usize) + i;
                if idx < self.nodes.len() {
                    self.nodes[idx] = OctreeNode::new_leaf(0, depth);
                }
            }
            return ptr;
        }

        let ptr = self.nodes.len() as u32;
        self.nodes
            .extend((0..8).map(|_| OctreeNode::new_leaf(0, depth)));
        ptr
    }

    /// Free 8 child nodes for later reuse
    #[inline]
    fn free_children(&mut self, child_ptr: u32) {
        self.free_list.push(child_ptr);
    }

    #[inline]
    fn get_child_index(pos: IVec3, mid: IVec3) -> usize {
        ((pos.x >= mid.x) as usize)
            | (((pos.y >= mid.y) as usize) << 1)
            | (((pos.z >= mid.z) as usize) << 2)
    }

    /// Set a single voxel in the octree
    pub fn set_voxel(&mut self, pos: IVec3, voxel_id: VoxelID) {
        let size = self.size as i32;
        if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.x >= size || pos.y >= size || pos.z >= size {
            return;
        }

        if self.set_voxel_recursive(0, pos, voxel_id, vec3(0, 0, 0), size, 0) {
            // Mark affected chunk as dirty instead of invalidating entire cache
            let chunk_pos = vec3(
                pos.x.div_euclid(CHUNK_SIZE as i32),
                pos.y.div_euclid(CHUNK_SIZE as i32),
                pos.z.div_euclid(CHUNK_SIZE as i32),
            );
            self.dirty_chunks.insert(chunk_pos);
        }
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

        // If node is a leaf
        if self.nodes[node_idx].is_leaf() {
            let current_voxel = self.nodes[node_idx].data;

            // Early termination if already correct value
            if current_voxel == voxel_id {
                return false;
            }

            // Need to split - allocate 8 children
            self.needs_full_gpu_rebuild = true;
            let child_ptr = self.allocate_children(current_depth + 1);

            // Initialize children with current value
            for i in 0..8 {
                self.nodes[(child_ptr as usize) + i].data = current_voxel;
                self.dirty_nodes.insert((child_ptr as usize) + i);
            }

            self.nodes[node_idx].child_ptr = child_ptr;
            self.nodes[node_idx].data = 0;
        }

        // Calculate child position
        let child_min = vec3(
            if pos.x >= mid.x { mid.x } else { node_min.x },
            if pos.y >= mid.y { mid.y } else { node_min.y },
            if pos.z >= mid.z { mid.z } else { node_min.z },
        );

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        let child_node_idx = child_ptr + child_idx;

        // Recurse into child
        let changed = self.set_voxel_recursive(
            child_node_idx,
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

    fn try_merge_children(&mut self, node_idx: usize) {
        if self.nodes[node_idx].is_leaf() {
            return;
        }

        let child_ptr = self.nodes[node_idx].child_ptr as usize;
        let first_child = &self.nodes[child_ptr];

        if !first_child.is_leaf() {
            return;
        }

        let first_value = first_child.data;

        // Check if all 8 children are identical leaves
        for i in 1..8 {
            let child = &self.nodes[child_ptr + i];
            if !child.is_leaf() || child.data != first_value {
                return;
            }
        }

        self.needs_full_gpu_rebuild = true;

        // Merge into parent
        let old_child_ptr = self.nodes[node_idx].child_ptr;
        self.nodes[node_idx].child_ptr = 0;
        self.nodes[node_idx].data = first_value;
        self.free_children(old_child_ptr);

        self.dirty_nodes.insert(node_idx);
    }

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

    /// Update an entire chunk region with lazy cache rebuilding
    pub fn update_chunk_region(
        &mut self,
        chunk_pos: IVec3,
        chunk_data: &[u8; 4096],
        offset: [i32; 3],
    ) {
        self.total_updates += 1;

        let octree_base = vec3(
            chunk_pos.x * CHUNK_SIZE as i32 - offset[0],
            chunk_pos.y * CHUNK_SIZE as i32 - offset[1],
            chunk_pos.z * CHUNK_SIZE as i32 - offset[2],
        );

        // Bounds check
        if octree_base.x < 0 || octree_base.y < 0 || octree_base.z < 0 {
            return;
        }

        let octree_end =
            octree_base + vec3(CHUNK_SIZE as i32, CHUNK_SIZE as i32, CHUNK_SIZE as i32);
        if octree_end.x > self.size as i32
            || octree_end.y > self.size as i32
            || octree_end.z > self.size as i32
        {
            return;
        }

        // Lazy cache building - only build for this specific chunk if needed
        if !self.spatial_cache.is_cached(&chunk_pos) {
            self.build_cache_for_chunk(chunk_pos);
        }

        let updated = if let Some(cached_nodes) = self.spatial_cache.get_nodes(&chunk_pos) {
            self.cache_hits += 1;
            self.update_with_cache(cached_nodes.clone(), chunk_data, octree_base)
        } else {
            self.update_without_cache(chunk_data, octree_base)
        };

        if updated {
            self.dirty_chunks.insert(chunk_pos);
        }
    }

    /// Build cache only for a specific chunk region (lazy/incremental)
    fn build_cache_for_chunk(&mut self, chunk_pos: IVec3) {
        // Calculate world bounds for this chunk
        let chunk_min = vec3(
            chunk_pos.x * CHUNK_SIZE as i32,
            chunk_pos.y * CHUNK_SIZE as i32,
            chunk_pos.z * CHUNK_SIZE as i32,
        );
        let chunk_max = chunk_min
            + vec3(
                CHUNK_SIZE as i32 - 1,
                CHUNK_SIZE as i32 - 1,
                CHUNK_SIZE as i32 - 1,
            );

        // Traverse tree and cache nodes that intersect this chunk
        self.cache_chunk_recursive(
            0,
            vec3(0, 0, 0),
            self.size as i32,
            0,
            chunk_pos,
            chunk_min,
            chunk_max,
        );
    }

    fn cache_chunk_recursive(
        &mut self,
        node_idx: usize,
        node_min: IVec3,
        node_size: i32,
        depth: u8,
        chunk_pos: IVec3,
        chunk_min: IVec3,
        chunk_max: IVec3,
    ) {
        let node_max = node_min + vec3(node_size - 1, node_size - 1, node_size - 1);

        // Check if node intersects the chunk bounds
        if node_max.x < chunk_min.x
            || node_min.x > chunk_max.x
            || node_max.y < chunk_min.y
            || node_min.y > chunk_max.y
            || node_max.z < chunk_min.z
            || node_min.z > chunk_max.z
        {
            return; // No intersection, skip this branch
        }

        let node = self.nodes[node_idx];

        // Cache this node for the chunk
        self.spatial_cache.add_node(
            chunk_pos,
            CachedNodeInfo {
                node_idx,
                node_min,
                node_size,
                depth,
            },
        );

        // Recurse to children if not a leaf
        if !node.is_leaf() {
            let half_size = node_size / 2;
            let child_ptr = node.child_ptr as usize;

            for i in 0..8 {
                let child_offset = vec3(
                    if i & 1 != 0 { half_size } else { 0 },
                    if i & 2 != 0 { half_size } else { 0 },
                    if i & 4 != 0 { half_size } else { 0 },
                );

                self.cache_chunk_recursive(
                    child_ptr + i,
                    node_min + child_offset,
                    half_size,
                    depth + 1,
                    chunk_pos,
                    chunk_min,
                    chunk_max,
                );
            }
        }
    }

    pub fn rebuild_dirty_cache(&mut self) {
        if self.dirty_chunks.is_empty() {
            return;
        }

        if self.dirty_chunks.len() > self.spatial_cache.dirty_threshold {
            let dirty: Vec<_> = self.dirty_chunks.drain().collect();
            for chunk_pos in dirty {
                self.spatial_cache.invalidate_chunk(chunk_pos);
            }
            return;
        }

        let dirty: Vec<_> = self.dirty_chunks.drain().collect();
        for chunk_pos in dirty {
            self.spatial_cache.invalidate_chunk(chunk_pos);
            self.build_cache_for_chunk(chunk_pos);
        }
    }

    fn update_with_cache(
        &mut self,
        cached_nodes: Vec<CachedNodeInfo>,
        chunk_data: &[u8; 4096],
        octree_base: IVec3,
    ) -> bool {
        let mut updated = false;

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let idx = x as usize
                        + y as usize * CHUNK_SIZE as usize
                        + z as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize;

                    let pos = octree_base + vec3(x as i32, y as i32, z as i32);
                    let new_voxel = chunk_data[idx];

                    // Find the right cached node for this position
                    for info in &cached_nodes {
                        if pos.x >= info.node_min.x
                            && pos.x < info.node_min.x + info.node_size
                            && pos.y >= info.node_min.y
                            && pos.y < info.node_min.y + info.node_size
                            && pos.z >= info.node_min.z
                            && pos.z < info.node_min.z + info.node_size
                        {
                            updated |= self.set_voxel_recursive(
                                info.node_idx,
                                pos,
                                new_voxel,
                                info.node_min,
                                info.node_size,
                                info.depth,
                            );
                            break;
                        }
                    }
                }
            }
        }

        updated
    }

    fn update_without_cache(&mut self, chunk_data: &[u8; 4096], octree_base: IVec3) -> bool {
        let mut updated = false;

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let idx = x as usize
                        + y as usize * CHUNK_SIZE as usize
                        + z as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize;

                    let pos = octree_base + vec3(x as i32, y as i32, z as i32);

                    updated |= self.set_voxel_recursive(
                        0,
                        pos,
                        chunk_data[idx],
                        vec3(0, 0, 0),
                        self.size as i32,
                        0,
                    );
                }
            }
        }

        updated
    }

    /// Clear a chunk region (set all voxels to 0)
    pub fn clear_chunk_region(&mut self, chunk_pos: IVec3, offset: [i32; 3]) {
        self.update_chunk_region(chunk_pos, &[0u8; 4096], offset);
    }

    /// Batch clear multiple chunks
    pub fn clear_chunks_batch(&mut self, positions: &[IVec3], offset: [i32; 3]) {
        let empty_data = [0u8; 4096];
        for pos in positions {
            self.update_chunk_region(*pos, &empty_data, offset);
        }
    }

    /// Compact the octree to remove fragmentation
    pub fn compact(&mut self) {
        let mut reachable = vec![false; self.nodes.len()];
        self.mark_reachable(0, &mut reachable);

        let new_size = reachable.iter().filter(|&&r| r).count();
        if new_size == self.nodes.len() {
            return;
        }

        // Build new index mapping
        let mut new_indices = vec![0u32; self.nodes.len()];
        let mut new_idx = 0u32;

        for (old_idx, &is_reachable) in reachable.iter().enumerate() {
            if is_reachable {
                new_indices[old_idx] = new_idx;
                new_idx += 1;
            }
        }

        // Create compacted node array
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

        // Invalidate all cache after compaction
        self.spatial_cache.clear();
        self.dirty_chunks.clear();
    }

    fn mark_reachable(&self, node_idx: usize, reachable: &mut [bool]) {
        if node_idx >= self.nodes.len() || reachable[node_idx] {
            return;
        }

        reachable[node_idx] = true;

        let node = &self.nodes[node_idx];
        if !node.is_leaf() {
            let child_ptr = node.child_ptr as usize;
            for i in 0..8 {
                self.mark_reachable(child_ptr + i, reachable);
            }
        }
    }

    pub fn get_stats(&self) -> OctreeStats {
        let mut stats = OctreeStats::default();
        stats.total_nodes = self.nodes.len();
        stats.free_nodes = self.free_list.len() * 8;
        stats.total_updates = self.total_updates;
        stats.cache_hits = self.cache_hits;
        stats.cache_hit_rate = if self.total_updates > 0 {
            (self.cache_hits as f32 / self.total_updates as f32) * 100.0
        } else {
            0.0
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

    pub fn get_dirty_ranges(&self) -> Vec<(usize, usize)> {
        if self.dirty_nodes.is_empty() {
            return Vec::new();
        }

        let mut sorted: Vec<usize> = self.dirty_nodes.iter().copied().collect();
        sorted.sort_unstable();

        let mut ranges = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0] + 1;

        for &idx in &sorted[1..] {
            if idx == end {
                // Contiguous - extend range
                end = idx + 1;
            } else if idx <= end + 8 {
                // Close enough - merge (within one octree node)
                end = idx + 1;
            } else {
                // Gap too large - start new range
                ranges.push((start, end));
                start = idx;
                end = idx + 1;
            }
        }
        ranges.push((start, end));

        ranges
    }

    pub fn needs_full_gpu_rebuild(&self) -> bool {
        self.needs_full_gpu_rebuild
    }

    pub fn clear_gpu_dirty_state(&mut self) {
        self.dirty_nodes.clear();
        self.needs_full_gpu_rebuild = false;
    }

    pub fn get_dirty_node_count(&self) -> usize {
        self.dirty_nodes.len()
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
    pub cache_hits: u64,
    pub cache_hit_rate: f32,
}

impl OctreeStats {
    pub fn memory_usage(&self) -> usize {
        self.total_nodes * std::mem::size_of::<OctreeNode>()
    }

    pub fn compression_ratio(&self, texture_size: u32) -> f32 {
        let texture_voxels = (texture_size * texture_size * texture_size) as usize;
        let octree_bytes = self.memory_usage();
        texture_voxels as f32 / octree_bytes as f32
    }
}
