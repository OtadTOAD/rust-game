use nalgebra_glm::{IVec3, Vec3};
use noise::{NoiseFn, Perlin, Seedable};

pub type VoxelID = u8;

pub trait WorldGenerator: Send + Sync {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID;
    fn name(&self) -> &str;
}

pub struct SphereGenerator {
    pub center: Vec3,
    pub radius: f32,
    pub voxel_id: VoxelID,
}

impl WorldGenerator for SphereGenerator {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID {
        let dx = x as f32 - self.center.x;
        let dy = y as f32 - self.center.y;
        let dz = z as f32 - self.center.z;
        let dist_sq = dx * dx + dy * dy + dz * dz;

        if dist_sq < self.radius * self.radius {
            self.voxel_id
        } else {
            0
        }
    }

    fn name(&self) -> &str {
        "Sphere"
    }
}

pub struct TerrainGenerator {
    noise: Perlin,
    scale: f64,
    height_multiplier: f32,
    base_height: i32,
    grass_id: VoxelID,
    dirt_id: VoxelID,
    stone_id: VoxelID,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            noise: Perlin::new(seed),
            scale: 0.02,
            height_multiplier: 40.0,
            base_height: 64,
            grass_id: 1,
            dirt_id: 2,
            stone_id: 3,
        }
    }

    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_height(mut self, multiplier: f32, base: i32) -> Self {
        self.height_multiplier = multiplier;
        self.base_height = base;
        self
    }
}

impl WorldGenerator for TerrainGenerator {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID {
        let noise_val = self
            .noise
            .get([x as f64 * self.scale, z as f64 * self.scale]);
        let height = self.base_height + (noise_val * self.height_multiplier as f64) as i32;

        if y > height {
            0
        } else if y == height {
            self.grass_id
        } else if y > height - 3 {
            self.dirt_id
        } else {
            self.stone_id
        }
    }

    fn name(&self) -> &str {
        "Terrain"
    }
}

pub struct CaveGenerator {
    terrain: TerrainGenerator,
    cave_noise: Perlin,
    cave_scale: f64,
    cave_threshold: f64,
}

impl CaveGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            terrain: TerrainGenerator::new(seed),
            cave_noise: Perlin::new(seed.wrapping_add(1)),
            cave_scale: 0.05,
            cave_threshold: 0.3,
        }
    }
}

impl WorldGenerator for CaveGenerator {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID {
        let terrain_voxel = self.terrain.generate(x, y, z);

        if terrain_voxel == 0 {
            return 0;
        }

        let cave_val = self.cave_noise.get([
            x as f64 * self.cave_scale,
            y as f64 * self.cave_scale,
            z as f64 * self.cave_scale,
        ]);

        if cave_val > self.cave_threshold {
            0
        } else {
            terrain_voxel
        }
    }

    fn name(&self) -> &str {
        "Caves"
    }
}

pub struct FlatGenerator {
    pub height: i32,
    pub voxel_id: VoxelID,
}

impl WorldGenerator for FlatGenerator {
    fn generate(&self, _x: i32, y: i32, _z: i32) -> VoxelID {
        if y <= self.height { self.voxel_id } else { 0 }
    }

    fn name(&self) -> &str {
        "Flat"
    }
}

pub struct CheckerboardGenerator {
    pub size: i32,
    pub voxel_a: VoxelID,
    pub voxel_b: VoxelID,
}

impl WorldGenerator for CheckerboardGenerator {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID {
        let sum = (x / self.size) + (y / self.size) + (z / self.size);
        if sum % 2 == 0 {
            self.voxel_a
        } else {
            self.voxel_b
        }
    }

    fn name(&self) -> &str {
        "Checkerboard"
    }
}

pub struct LayeredGenerator {
    layers: Vec<Box<dyn WorldGenerator>>,
}

impl LayeredGenerator {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn add_layer(mut self, generator: Box<dyn WorldGenerator>) -> Self {
        self.layers.push(generator);
        self
    }
}

impl WorldGenerator for LayeredGenerator {
    fn generate(&self, x: i32, y: i32, z: i32) -> VoxelID {
        for layer in self.layers.iter().rev() {
            let voxel = layer.generate(x, y, z);
            if voxel != 0 {
                return voxel;
            }
        }
        0
    }

    fn name(&self) -> &str {
        "Layered"
    }
}

pub fn create_sphere_world(center: Vec3, radius: f32) -> Box<dyn WorldGenerator> {
    Box::new(SphereGenerator {
        center,
        radius,
        voxel_id: 1,
    })
}

pub fn create_terrain_world(seed: u32) -> Box<dyn WorldGenerator> {
    Box::new(TerrainGenerator::new(seed))
}

pub fn create_cave_world(seed: u32) -> Box<dyn WorldGenerator> {
    Box::new(CaveGenerator::new(seed))
}

pub fn create_flat_world(height: i32) -> Box<dyn WorldGenerator> {
    Box::new(FlatGenerator {
        height,
        voxel_id: 1,
    })
}
