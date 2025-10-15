use nalgebra_glm::{Vec3, vec3};
use std::collections::{HashMap, VecDeque};

use crate::engine::voxel_chunk::{CHUNK_SIZE, ChunkPosition, ChunkUpdateType, VoxelChunk};

#[derive(Clone, Debug)]
pub struct ChunkUpdate {
    pub position: ChunkPosition,
    pub update_type: ChunkUpdateType,
    pub priority: u32,
}

pub struct VoxelWorld {
    pub chunks: HashMap<ChunkPosition, VoxelChunk>,
    pub render_dist: i16,
    last_center_chunk: Option<ChunkPosition>,

    pub update_queue: VecDeque<ChunkUpdate>,
    pub pending_removals: Vec<ChunkPosition>,
    current_frame: u64,
}

impl VoxelWorld {
    pub fn new(render_dist: i16) -> Self {
        Self {
            chunks: HashMap::new(),
            render_dist,
            last_center_chunk: None,
            update_queue: VecDeque::new(),
            pending_removals: Vec::new(),
            current_frame: 0,
        }
    }

    pub fn load_chunks(&mut self, pos: Vec3) -> bool {
        self.current_frame += 1;

        let main_chunk: ChunkPosition = vec3(
            (pos.x / CHUNK_SIZE as f32).floor() as i32,
            (pos.y / CHUNK_SIZE as f32).floor() as i32,
            (pos.z / CHUNK_SIZE as f32).floor() as i32,
        );

        let chunks_changed = if let Some(last) = self.last_center_chunk {
            last != main_chunk
        } else {
            true
        };

        if !chunks_changed {
            return false;
        }

        self.last_center_chunk = Some(main_chunk);
        let mut any_changes = false;

        for dx in -self.render_dist..=self.render_dist {
            for dy in -self.render_dist..=self.render_dist {
                for dz in -self.render_dist..=self.render_dist {
                    let chunk_pos = vec3(
                        main_chunk.x + dx as i32,
                        main_chunk.y + dy as i32,
                        main_chunk.z + dz as i32,
                    );

                    if !self.chunks.contains_key(&chunk_pos) {
                        let mut new_chunk = VoxelChunk::new(chunk_pos);
                        new_chunk.load();

                        let dist_sq = (dx * dx + dy * dy + dz * dz) as u32;
                        let priority = 1000 - dist_sq.min(999);

                        self.chunks.insert(chunk_pos, new_chunk);
                        self.update_queue.push_back(ChunkUpdate {
                            position: chunk_pos,
                            update_type: ChunkUpdateType::Added,
                            priority,
                        });

                        any_changes = true;
                    }
                }
            }
        }

        let chunks_to_remove: Vec<ChunkPosition> = self
            .chunks
            .keys()
            .filter(|&&pos| {
                let dx = (pos.x - main_chunk.x).abs();
                let dy = (pos.y - main_chunk.y).abs();
                let dz = (pos.z - main_chunk.z).abs();
                dx > self.render_dist as i32
                    || dy > self.render_dist as i32
                    || dz > self.render_dist as i32
            })
            .copied()
            .collect();

        for pos in chunks_to_remove {
            self.pending_removals.push(pos);
            self.chunks.remove(&pos);
            any_changes = true;
        }

        if any_changes {
            self.sort_update_queue();
        }

        any_changes
    }

    fn sort_update_queue(&mut self) {
        let mut items: Vec<_> = self.update_queue.drain(..).collect();
        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.update_queue = items.into();
    }

    pub fn get_pending_updates(
        &mut self,
        max_count: usize,
    ) -> (Vec<ChunkUpdate>, Vec<ChunkPosition>) {
        let mut updates = Vec::new();

        for _ in 0..max_count {
            if let Some(update) = self.update_queue.pop_front() {
                if let Some(chunk) = self.chunks.get_mut(&update.position) {
                    if chunk.needs_update() {
                        updates.push(update);
                        chunk.mark_update_processed();
                    }
                }
            } else {
                break;
            }
        }

        let removals = std::mem::take(&mut self.pending_removals);

        (updates, removals)
    }

    pub fn get_chunk_voxels(&self, pos: &ChunkPosition) -> Option<[u8; 4096]> {
        self.chunks.get(pos).map(|chunk| {
            let mut data = [0u8; 4096];
            data.copy_from_slice(&chunk.voxels[..4096]);
            data
        })
    }

    pub fn has_pending_updates(&self) -> bool {
        !self.update_queue.is_empty() || !self.pending_removals.is_empty()
    }

    pub fn pending_update_count(&self) -> usize {
        self.update_queue.len() + self.pending_removals.len()
    }

    pub fn get_world_bounds(&self) -> (ChunkPosition, ChunkPosition, [u32; 3]) {
        if self.chunks.is_empty() {
            return (vec3(0, 0, 0), vec3(0, 0, 0), [0, 0, 0]);
        }

        let min_x = self.chunks.keys().map(|p| p.x).min().unwrap();
        let min_y = self.chunks.keys().map(|p| p.y).min().unwrap();
        let min_z = self.chunks.keys().map(|p| p.z).min().unwrap();

        let max_x = self.chunks.keys().map(|p| p.x).max().unwrap();
        let max_y = self.chunks.keys().map(|p| p.y).max().unwrap();
        let max_z = self.chunks.keys().map(|p| p.z).max().unwrap();

        let world_size_x = (max_x - min_x + 1) as u32 * CHUNK_SIZE as u32;
        let world_size_y = (max_y - min_y + 1) as u32 * CHUNK_SIZE as u32;
        let world_size_z = (max_z - min_z + 1) as u32 * CHUNK_SIZE as u32;

        (
            vec3(min_x, min_y, min_z),
            vec3(max_x, max_y, max_z),
            [world_size_x, world_size_y, world_size_z],
        )
    }
}
