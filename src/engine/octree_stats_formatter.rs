use super::octree::OctreeStats;

pub struct OctreeStatsFormatter;

impl OctreeStatsFormatter {
    /// Print detailed statistics about the octree with performance metrics
    pub fn print_stats(stats: &OctreeStats, world_size: [u32; 3]) {
        let texture_voxels = (world_size[0] * world_size[1] * world_size[2]) as usize;
        let texture_bytes = texture_voxels;

        println!("\n╔═══════════════════════════════════════╗");
        println!("║       Octree Statistics               ║");
        println!("╠═══════════════════════════════════════╣");

        // Node statistics
        println!("║ Nodes:                                ║");
        println!("║   Total:        {:>20} ║", stats.total_nodes);
        println!(
            "║   Active:       {:>20} ║",
            stats.total_nodes - stats.free_nodes
        );
        println!("║   Free:         {:>20} ║", stats.free_nodes);
        println!("║   Branch:       {:>20} ║", stats.branch_nodes);
        println!("║   Leaf:         {:>20} ║", stats.leaf_nodes);
        println!("║   Filled leaf:  {:>20} ║", stats.filled_leaf_nodes);
        println!("║   Empty leaf:   {:>20} ║", stats.empty_leaf_nodes);
        println!("║   Max depth:    {:>20} ║", stats.max_depth);
        println!("║                                       ║");

        // Memory statistics
        let octree_mb = stats.memory_usage() as f32 / (1024.0 * 1024.0);
        let texture_mb = texture_bytes as f32 / (1024.0 * 1024.0);
        let compression = stats.compression_ratio(world_size[0]);
        let fragmentation = (stats.free_nodes as f32 / stats.total_nodes.max(1) as f32) * 100.0;

        println!("║ Memory:                               ║");
        println!("║   Octree:       {:>15} bytes ║", stats.memory_usage());
        println!("║                 {:>15.2} MB   ║", octree_mb);
        println!("║   3D Texture:   {:>15} bytes ║", texture_bytes);
        println!("║                 {:>15.2} MB   ║", texture_mb);
        println!("║   Fragmentation:{:>17.1}%   ║", fragmentation);
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

        // Efficiency metrics
        let efficiency = (stats.filled_leaf_nodes as f32 / stats.leaf_nodes.max(1) as f32) * 100.0;
        let sparsity = (stats.empty_leaf_nodes as f32 / stats.leaf_nodes.max(1) as f32) * 100.0;

        println!("║ Efficiency:                           ║");
        println!("║   Leaf fill:    {:>17.1}%   ║", efficiency);
        println!("║   Sparsity:     {:>17.1}%   ║", sparsity);

        // Performance metrics (if available)
        if stats.total_updates > 0 {
            println!("║                                       ║");
            println!("║ Performance:                          ║");
            println!("║   Total updates:{:>20} ║", stats.total_updates);
            println!("║   Cache hits:   {:>20} ║", stats.cache_hits);
            println!("║   Hit rate:     {:>17.1}%   ║", stats.cache_hit_rate);

            if stats.cache_hit_rate >= 80.0 {
                println!("║   🚀 Excellent cache performance!     ║");
            } else if stats.cache_hit_rate >= 60.0 {
                println!("║   ✓ Good cache performance           ║");
            } else if stats.cache_hit_rate >= 40.0 {
                println!("║   ⚠️  Moderate cache performance      ║");
            } else {
                println!("║   ❌ Poor cache performance           ║");
            }
        }

        println!("╚═══════════════════════════════════════╝\n");

        // Recommendations
        if fragmentation > 30.0 {
            println!(
                "💡 Recommendation: Consider compacting the octree to reclaim {} free nodes",
                stats.free_nodes
            );
        }

        if compression < 1.0 {
            println!("⚠️  Warning: Octree is using more memory than a 3D texture");
            println!("   This may indicate a dense voxel world or frequent small changes");
        }

        if stats.total_updates > 0 && stats.cache_hit_rate < 50.0 {
            println!("⚠️  Warning: Low cache hit rate detected");
            println!("   Consider rebuilding the spatial cache or reducing update frequency");
        }

        println!();
    }

    /// Print a compact one-line summary
    pub fn print_summary(stats: &OctreeStats, world_size: [u32; 3]) {
        let compression = stats.compression_ratio(world_size[0]);
        let fragmentation = (stats.free_nodes as f32 / stats.total_nodes.max(1) as f32) * 100.0;

        print!(
            "Octree: {} nodes ({} active, {} free), ",
            stats.total_nodes,
            stats.total_nodes - stats.free_nodes,
            stats.free_nodes
        );
        print!(
            "{:.2}x compression, {:.1}% fragmentation",
            compression, fragmentation
        );

        if stats.total_updates > 0 {
            print!(", {:.1}% cache hits", stats.cache_hit_rate);
        }

        println!();
    }

    /// Print comparison between two stats (useful for before/after comparisons)
    pub fn print_comparison(before: &OctreeStats, after: &OctreeStats, operation: &str) {
        println!("\n╔═══════════════════════════════════════╗");
        println!("║  {} Comparison", operation);
        println!("╠═══════════════════════════════════════╣");

        let node_delta = after.total_nodes as i64 - before.total_nodes as i64;
        let memory_delta = after.memory_usage() as i64 - before.memory_usage() as i64;
        let memory_delta_mb = memory_delta as f32 / (1024.0 * 1024.0);

        println!("║ Total Nodes:                          ║");
        println!("║   Before:       {:>20} ║", before.total_nodes);
        println!("║   After:        {:>20} ║", after.total_nodes);
        if node_delta != 0 {
            println!("║   Change:       {:>+20} ║", node_delta);
        }
        println!("║                                       ║");

        println!("║ Memory Usage:                         ║");
        println!(
            "║   Before:       {:>15.2} MB   ║",
            before.memory_usage() as f32 / (1024.0 * 1024.0)
        );
        println!(
            "║   After:        {:>15.2} MB   ║",
            after.memory_usage() as f32 / (1024.0 * 1024.0)
        );
        if memory_delta != 0 {
            println!("║   Change:       {:>+14.2} MB   ║", memory_delta_mb);
        }
        println!("║                                       ║");

        println!("║ Free Nodes:                           ║");
        println!("║   Before:       {:>20} ║", before.free_nodes);
        println!("║   After:        {:>20} ║", after.free_nodes);

        let frag_before = (before.free_nodes as f32 / before.total_nodes.max(1) as f32) * 100.0;
        let frag_after = (after.free_nodes as f32 / after.total_nodes.max(1) as f32) * 100.0;
        println!("║   Fragmentation:{:>17.1}%   ║", frag_before);
        println!("║                 {:>17.1}%   ║", frag_after);

        println!("╚═══════════════════════════════════════╝\n");

        // Verdict
        if node_delta < 0 {
            println!("✓ Reduced node count by {}", -node_delta);
        }
        if memory_delta < 0 {
            println!("✓ Saved {:.2} MB of memory", -memory_delta_mb);
        }
        if frag_after < frag_before {
            println!(
                "✓ Reduced fragmentation by {:.1}%",
                frag_before - frag_after
            );
        }

        println!();
    }

    /// Print a progress indicator for long operations
    pub fn print_progress(operation: &str, current: usize, total: usize) {
        let percentage = (current as f32 / total.max(1) as f32) * 100.0;
        let bar_width = 40;
        let filled = ((percentage / 100.0) * bar_width as f32) as usize;

        print!("\r{}: [", operation);
        for i in 0..bar_width {
            if i < filled {
                print!("█");
            } else {
                print!("░");
            }
        }
        print!("] {:.1}% ({}/{})", percentage, current, total);

        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        if current >= total {
            println!(); // New line when complete
        }
    }
}
