use nalgebra_glm::{IVec3, vec3};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::engine::unified_octree::{CHUNK_SIZE, UnifiedOctree};
use crate::engine::world_generator::WorldGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world_pos(world_pos: IVec3) -> Self {
        Self {
            x: world_pos.x.div_euclid(CHUNK_SIZE),
            y: world_pos.y.div_euclid(CHUNK_SIZE),
            z: world_pos.z.div_euclid(CHUNK_SIZE),
        }
    }

    pub fn to_world_min(&self) -> IVec3 {
        vec3(
            self.x * CHUNK_SIZE,
            self.y * CHUNK_SIZE,
            self.z * CHUNK_SIZE,
        )
    }

    pub fn distance_sq(&self, other: &ChunkPos) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

pub struct ChunkManager {
    loaded_chunks: HashSet<ChunkPos>,
    generator: Arc<dyn WorldGenerator>,
    render_distance: i32,
    vertical_render_distance: i32,
    last_center: Option<ChunkPos>,

    chunks_loaded_this_frame: usize,
    chunks_unloaded_this_frame: usize,
    max_chunks_per_frame: usize,

    frames_since_update: u32,
    update_every_n_frames: u32,
}

impl ChunkManager {
    pub fn new(generator: Arc<dyn WorldGenerator>, render_distance: i32) -> Self {
        Self {
            loaded_chunks: HashSet::new(),
            generator,
            render_distance,
            vertical_render_distance: render_distance / 2,
            last_center: None,
            chunks_loaded_this_frame: 0,
            chunks_unloaded_this_frame: 0,
            max_chunks_per_frame: 8,
            frames_since_update: 0,
            update_every_n_frames: 3,
        }
    }

    pub fn update(&mut self, camera_pos: IVec3, octree: &mut UnifiedOctree) -> bool {
        let center_chunk = ChunkPos::from_world_pos(camera_pos);

        self.frames_since_update += 1;
        let is_first_update = self.last_center.is_none();

        if self.last_center == Some(center_chunk) && !is_first_update {
            if self.frames_since_update < self.update_every_n_frames {
                return false;
            }
        }

        self.frames_since_update = 0;

        let start_time = std::time::Instant::now();
        self.chunks_loaded_this_frame = 0;
        self.chunks_unloaded_this_frame = 0;

        let desired_chunks = self.get_chunks_in_range(center_chunk);

        let chunks_to_unload: Vec<_> = self
            .loaded_chunks
            .iter()
            .filter(|chunk| !desired_chunks.contains(chunk))
            .copied()
            .collect();

        for chunk in chunks_to_unload {
            self.unload_chunk(chunk, octree);
            self.chunks_unloaded_this_frame += 1;
        }

        let mut chunks_to_load: Vec<_> = desired_chunks
            .iter()
            .filter(|chunk| !self.loaded_chunks.contains(chunk))
            .copied()
            .collect();

        chunks_to_load.sort_by_key(|chunk| chunk.distance_sq(&center_chunk));

        let load_limit = if is_first_update {
            chunks_to_load.len()
        } else {
            self.max_chunks_per_frame
        };

        for chunk in chunks_to_load.iter().take(load_limit) {
            self.load_chunk(*chunk, octree);
            self.chunks_loaded_this_frame += 1;
        }

        self.last_center = Some(center_chunk);

        let elapsed = start_time.elapsed();
        if is_first_update
            || self.chunks_loaded_this_frame > 0
            || self.chunks_unloaded_this_frame > 0
        {
            println!(
                "\n{} @ {:?} ({:?})",
                if is_first_update {
                    "Initial Load"
                } else {
                    "Chunk Update"
                },
                center_chunk,
                elapsed
            );
            println!(
                "  Loaded: {} | Unloaded: {} | Total: {}",
                self.chunks_loaded_this_frame,
                self.chunks_unloaded_this_frame,
                self.loaded_chunks.len()
            );
        }

        self.chunks_loaded_this_frame > 0 || self.chunks_unloaded_this_frame > 0
    }

    fn get_chunks_in_range(&self, center: ChunkPos) -> HashSet<ChunkPos> {
        let mut chunks = HashSet::new();

        for dy in -self.vertical_render_distance..=self.vertical_render_distance {
            for dx in -self.render_distance..=self.render_distance {
                for dz in -self.render_distance..=self.render_distance {
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    let max_dist_sq = self.render_distance * self.render_distance;

                    if dist_sq <= max_dist_sq {
                        chunks.insert(ChunkPos::new(center.x + dx, center.y + dy, center.z + dz));
                    }
                }
            }
        }

        chunks
    }

    fn load_chunk(&mut self, chunk: ChunkPos, octree: &mut UnifiedOctree) {
        let world_min = chunk.to_world_min();
        let world_max = world_min + vec3(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1);

        let generator = Arc::clone(&self.generator);

        octree.set_region(world_min, world_max, |x, y, z| generator.generate(x, y, z));

        self.loaded_chunks.insert(chunk);
    }

    fn unload_chunk(&mut self, chunk: ChunkPos, octree: &mut UnifiedOctree) {
        let world_min = chunk.to_world_min();
        let world_max = world_min + vec3(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1);

        octree.set_region(world_min, world_max, |_, _, _| 0);

        self.loaded_chunks.remove(&chunk);
    }

    pub fn is_chunk_loaded(&self, chunk: ChunkPos) -> bool {
        self.loaded_chunks.contains(&chunk)
    }

    pub fn get_loaded_count(&self) -> usize {
        self.loaded_chunks.len()
    }

    pub fn set_render_distance(&mut self, distance: i32) {
        self.render_distance = distance;
        self.vertical_render_distance = distance / 2;
        self.last_center = None;
    }

    pub fn set_max_chunks_per_frame(&mut self, max: usize) {
        self.max_chunks_per_frame = max;
    }

    pub fn set_update_frequency(&mut self, frames: u32) {
        self.update_every_n_frames = frames;
    }

    pub fn get_stats(&self) -> String {
        format!(
            "Chunks: {} | RD: {}",
            self.loaded_chunks.len(),
            self.render_distance
        )
    }
}

pub struct ChunkCache {
    cache: HashMap<ChunkPos, Vec<u8>>,
    max_size: usize,
}

impl ChunkCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&self, pos: ChunkPos) -> Option<&Vec<u8>> {
        self.cache.get(&pos)
    }

    pub fn insert(&mut self, pos: ChunkPos, data: Vec<u8>) {
        if self.cache.len() >= self.max_size {
            if let Some(&key) = self.cache.keys().next() {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(pos, data);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
