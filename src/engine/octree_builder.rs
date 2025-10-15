use nalgebra_glm::vec3;

use super::octree::{Octree, OctreeStats, VoxelID};
use super::voxel_chunk::CHUNK_SIZE;
use super::voxel_world::VoxelWorld;

pub struct OctreeBuilder;

impl OctreeBuilder {
    /// Build an octree from a VoxelWorld using bottom-up construction
    /// This properly handles uniform regions!
    pub fn from_voxel_world(voxel_world: &VoxelWorld) -> (Octree, [i32; 3]) {
        let (min_pos, _max_pos, world_size) = voxel_world.get_world_bounds();

        // Calculate the next power of 2 that fits the world
        let max_dim = world_size[0].max(world_size[1]).max(world_size[2]);
        let octree_size = max_dim.next_power_of_two();

        println!("Building octree:");
        println!("  World size: {:?}", world_size);
        println!("  Octree size: {}", octree_size);
        println!(
            "  World offset: [{}, {}, {}]",
            min_pos.x, min_pos.y, min_pos.z
        );

        // Calculate offset - this converts octree coords to world coords
        let offset = [
            min_pos.x * CHUNK_SIZE as i32,
            min_pos.y * CHUNK_SIZE as i32,
            min_pos.z * CHUNK_SIZE as i32,
        ];

        // Count voxels for statistics
        let mut total_voxels = 0;
        let mut non_empty_voxels = 0;

        for (_chunk_pos, chunk) in &voxel_world.chunks {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        total_voxels += 1;
                        if chunk.get(x, y, z) != 0 {
                            non_empty_voxels += 1;
                        }
                    }
                }
            }
        }

        println!("  Total voxels: {}", total_voxels);
        println!(
            "  Non-empty voxels: {} ({:.1}% filled)",
            non_empty_voxels,
            (non_empty_voxels as f32 / total_voxels as f32) * 100.0
        );

        // Build octree using bottom-up construction
        println!("  Building from bottom-up...");
        let octree = Self::build_bottom_up(voxel_world, octree_size, offset);

        println!("  ✓ Octree built successfully");

        (octree, offset)
    }

    /// Bottom-up octree construction - builds leaves first, then merges upward
    fn build_bottom_up(voxel_world: &VoxelWorld, octree_size: u32, offset: [i32; 3]) -> Octree {
        let mut octree = Octree::new(octree_size);
        let depth = octree_size.trailing_zeros() as u8;

        // Create a lookup function for voxel data
        // FIXED: Correct coordinate transformation
        let get_voxel = |x: i32, y: i32, z: i32| -> VoxelID {
            // x, y, z are in octree space [0, octree_size)
            // Convert to world coordinates by adding offset
            let world_x = x + offset[0];
            let world_y = y + offset[1];
            let world_z = z + offset[2];

            // Find which chunk this belongs to
            let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
            let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
            let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);

            let chunk_pos = nalgebra_glm::vec3(chunk_x, chunk_y, chunk_z);

            if let Some(chunk) = voxel_world.chunks.get(&chunk_pos) {
                let local_x = world_x.rem_euclid(CHUNK_SIZE as i32) as u8;
                let local_y = world_y.rem_euclid(CHUNK_SIZE as i32) as u8;
                let local_z = world_z.rem_euclid(CHUNK_SIZE as i32) as u8;
                chunk.get(local_x, local_y, local_z)
            } else {
                0 // Empty if chunk doesn't exist
            }
        };

        // Build recursively from root
        Self::build_node_recursive(
            &mut octree,
            0,
            vec3(0, 0, 0),
            octree_size as i32,
            0,
            depth,
            &get_voxel,
        );

        octree
    }

    /// Recursively build a node and its children
    fn build_node_recursive<F>(
        octree: &mut Octree,
        node_idx: usize,
        node_min: nalgebra_glm::IVec3,
        node_size: i32,
        current_depth: u8,
        max_depth: u8,
        get_voxel: &F,
    ) where
        F: Fn(i32, i32, i32) -> VoxelID,
    {
        // Base case: at maximum depth or single voxel, just read the voxel
        if current_depth >= max_depth || node_size == 1 {
            let voxel = get_voxel(node_min.x, node_min.y, node_min.z);
            octree.nodes[node_idx].data = voxel;
            return;
        }

        // FIXED: More thorough uniformity check
        // Check if entire region is uniform
        let first_voxel = get_voxel(node_min.x, node_min.y, node_min.z);
        let mut is_uniform = true;

        // For small regions, check every voxel
        // For large regions, use adaptive sampling
        let sample_step = if node_size <= 8 {
            1
        } else if node_size <= 32 {
            2
        } else {
            4
        };

        'outer: for z in (0..node_size).step_by(sample_step as usize) {
            for y in (0..node_size).step_by(sample_step as usize) {
                for x in (0..node_size).step_by(sample_step as usize) {
                    let sample_x = node_min.x + x;
                    let sample_y = node_min.y + y;
                    let sample_z = node_min.z + z;

                    if get_voxel(sample_x, sample_y, sample_z) != first_voxel {
                        is_uniform = false;
                        break 'outer;
                    }
                }
            }
        }

        // If uniform based on samples, this is a leaf
        if is_uniform {
            octree.nodes[node_idx].data = first_voxel;
            return;
        }

        // Not uniform - need to subdivide
        let half_size = node_size / 2;
        let child_ptr = octree.nodes.len() as u32;

        // Create 8 children
        for _ in 0..8 {
            octree
                .nodes
                .push(super::octree::OctreeNode::new_leaf(0, current_depth + 1));
        }

        // Update parent to point to children
        octree.nodes[node_idx].child_ptr = child_ptr;
        octree.nodes[node_idx].data = 0;

        // Recursively build each child
        for child_idx in 0..8 {
            let child_offset = vec3(
                if child_idx & 1 != 0 { half_size } else { 0 },
                if child_idx & 2 != 0 { half_size } else { 0 },
                if child_idx & 4 != 0 { half_size } else { 0 },
            );

            let child_min = node_min + child_offset;

            Self::build_node_recursive(
                octree,
                (child_ptr as usize) + child_idx,
                child_min,
                half_size,
                current_depth + 1,
                max_depth,
                get_voxel,
            );
        }

        // After building children, check if they're all the same
        let all_leaves = (0..8).all(|i| octree.nodes[(child_ptr as usize) + i].is_leaf());

        if all_leaves {
            let first_child_value = octree.nodes[child_ptr as usize].data;
            let all_same =
                (1..8).all(|i| octree.nodes[(child_ptr as usize) + i].data == first_child_value);

            if all_same {
                // All children are identical - collapse into parent
                octree.nodes[node_idx].child_ptr = 0;
                octree.nodes[node_idx].data = first_child_value;
            }
        }
    }

    /// Verify that the octree matches the original voxel world
    pub fn verify_octree(
        voxel_world: &VoxelWorld,
        octree: &Octree,
        offset: [i32; 3],
    ) -> Result<(), String> {
        println!("\nVerifying octree integrity...");

        let mut mismatches = 0;
        let mut checked = 0;
        let mut sample_count = 0;
        const MAX_SAMPLES: i32 = 5;

        for (chunk_pos, chunk) in &voxel_world.chunks {
            let chunk_world_x = chunk_pos.x * CHUNK_SIZE as i32;
            let chunk_world_y = chunk_pos.y * CHUNK_SIZE as i32;
            let chunk_world_z = chunk_pos.z * CHUNK_SIZE as i32;

            // Sample verification
            let step = if voxel_world.chunks.len() > 100 { 4 } else { 2 };

            for z in (0..CHUNK_SIZE).step_by(step) {
                for y in (0..CHUNK_SIZE).step_by(step) {
                    for x in (0..CHUNK_SIZE).step_by(step) {
                        let expected = chunk.get(x, y, z);

                        // FIXED: Correct coordinate transformation for verification
                        let world_x = chunk_world_x + x as i32;
                        let world_y = chunk_world_y + y as i32;
                        let world_z = chunk_world_z + z as i32;

                        // Convert world coords to octree coords
                        let octree_x = world_x - offset[0];
                        let octree_y = world_y - offset[1];
                        let octree_z = world_z - offset[2];

                        if octree_x < 0
                            || octree_y < 0
                            || octree_z < 0
                            || octree_x >= octree.size as i32
                            || octree_y >= octree.size as i32
                            || octree_z >= octree.size as i32
                        {
                            continue;
                        }

                        let actual = octree.get_voxel(vec3(octree_x, octree_y, octree_z));

                        if expected != actual {
                            mismatches += 1;
                            if sample_count < MAX_SAMPLES {
                                println!(
                                    "  ✗ Mismatch at octree[{}, {}, {}] (world[{}, {}, {}]): expected {}, got {}",
                                    octree_x,
                                    octree_y,
                                    octree_z,
                                    world_x,
                                    world_y,
                                    world_z,
                                    expected,
                                    actual
                                );
                                sample_count += 1;
                            }
                        }

                        checked += 1;
                    }
                }
            }
        }

        if mismatches > 0 {
            Err(format!(
                "Verification failed: {} mismatches out of {} voxels checked",
                mismatches, checked
            ))
        } else {
            println!("  ✓ Verification passed: {} voxels checked", checked);
            Ok(())
        }
    }

    /// Print detailed statistics about the octree
    pub fn print_stats(stats: &OctreeStats, world_size: [u32; 3]) {
        let texture_voxels = (world_size[0] * world_size[1] * world_size[2]) as usize;
        let texture_bytes = texture_voxels;

        println!("\n╔═══════════════════════════════════════╗");
        println!("║       Octree Statistics               ║");
        println!("╠═══════════════════════════════════════╣");

        println!("║ Nodes:                                ║");
        println!("║   Total:        {:>20} ║", stats.total_nodes);
        println!("║   Branch:       {:>20} ║", stats.branch_nodes);
        println!("║   Leaf:         {:>20} ║", stats.leaf_nodes);
        println!("║   Filled leaf:  {:>20} ║", stats.filled_leaf_nodes);
        println!("║   Empty leaf:   {:>20} ║", stats.empty_leaf_nodes);
        println!("║   Max depth:    {:>20} ║", stats.max_depth);
        println!("║                                       ║");

        let octree_mb = stats.memory_usage() as f32 / (1024.0 * 1024.0);
        let texture_mb = texture_bytes as f32 / (1024.0 * 1024.0);
        let compression = stats.compression_ratio(world_size[0]);

        println!("║ Memory:                               ║");
        println!("║   Octree:       {:>15} bytes ║", stats.memory_usage());
        println!("║                 {:>15.2} MB   ║", octree_mb);
        println!("║   3D Texture:   {:>15} bytes ║", texture_bytes);
        println!("║                 {:>15.2} MB   ║", texture_mb);
        println!("║                                       ║");

        if compression > 1.0 {
            println!("║   🎉 Compression: {:>7.2}x SMALLER ║", compression);
            let saved = texture_mb - octree_mb;
            println!("║   💾 Saved:       {:>11.2} MB   ║", saved);
        } else {
            println!("║   ⚠️  Compression: {:>7.2}x LARGER  ║", 1.0 / compression);
            let overhead = octree_mb - texture_mb;
            println!("║   ❌ Overhead:     {:>11.2} MB   ║", overhead);
        }

        println!("║                                       ║");
        let efficiency = (stats.filled_leaf_nodes as f32 / stats.leaf_nodes.max(1) as f32) * 100.0;
        let sparsity = (stats.empty_leaf_nodes as f32 / stats.leaf_nodes.max(1) as f32) * 100.0;
        println!("║ Efficiency:                           ║");
        println!("║   Leaf fill:    {:>17.1}%   ║", efficiency);
        println!("║   Sparsity:     {:>17.1}%   ║", sparsity);
        println!("╚═══════════════════════════════════════╝\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::VoxelWorld;
    use nalgebra_glm::vec3;

    #[test]
    fn test_octree_builder() {
        let mut world = VoxelWorld::new(2);

        // Load some chunks
        world.load_chunks(vec3(0.0, 0.0, 0.0));

        // Build octree
        let (octree, offset) = OctreeBuilder::from_voxel_world(&world);

        println!("Built octree with offset: {:?}", offset);

        // Verify
        let result = OctreeBuilder::verify_octree(&world, &octree, offset);
        assert!(result.is_ok(), "Verification failed: {:?}", result);

        // Print stats
        let stats = octree.get_stats();
        let (_min, _max, size) = world.get_world_bounds();
        OctreeBuilder::print_stats(&stats, size);
    }
}
