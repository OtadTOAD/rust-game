#version 450

// Octree node structure (must match GpuOctreeNode - 8 bytes)
struct OctreeNode {
    uint child_ptr;  // 0 = leaf, otherwise index to first child
    uint data;       // voxel ID for leaves (packed in low 8 bits)
};

// Octree buffers
layout(set = 0, binding = 0) readonly buffer OctreeNodes {
    OctreeNode nodes[];
} octree_nodes;

layout(set = 0, binding = 1) uniform OctreeMetadata {
    uint octree_size;
    uint node_count;
    uint max_depth;
    uint _padding;
} octree_meta;

layout(set = 0, binding = 2) uniform CameraUBO {
    mat4 inv_proj;
    mat4 inv_view;
    vec4 cam_pos_and_scale;
    vec2 resolution;
} camera;

layout(location = 0) out vec4 out_color;

vec3 ray_direction(vec2 uv) {
    vec4 clip = vec4(uv * 2.0 - 1.0, -1.0, 1.0);
    vec4 view = camera.inv_proj * clip;
    view /= view.w;
    vec3 world_dir = (camera.inv_view * vec4(view.xyz, 0.0)).xyz;
    return normalize(world_dir);
}

bool intersect_box(vec3 ray_origin, vec3 ray_dir, vec3 box_min, vec3 box_max, 
                   out float t_near, out float t_far) {
    vec3 inv_dir = 1.0 / ray_dir;
    vec3 t_min = (box_min - ray_origin) * inv_dir;
    vec3 t_max = (box_max - ray_origin) * inv_dir;
    
    vec3 t1 = min(t_min, t_max);
    vec3 t2 = max(t_min, t_max);
    
    t_near = max(max(t1.x, t1.y), t1.z);
    t_far = min(min(t2.x, t2.y), t2.z);
    
    if (t_near < 0.0) {
        t_near = 0.0;
    }
    
    return t_far > t_near && t_far > 0.0;
}

vec3 get_voxel_color(uint voxel_id) {
    if (voxel_id == 1u) {
        return vec3(0.85, 0.35, 0.25);
    } else if (voxel_id == 2u) {
        return vec3(0.3, 0.7, 0.3);
    } else if (voxel_id == 3u) {
        return vec3(0.3, 0.5, 0.9);
    }
    return vec3(1.0, 0.0, 1.0);
}

// Stack entry for octree traversal
struct StackEntry {
    uint node_idx;
    vec3 box_min;
    vec3 box_max;
    float t_enter;
};

// FIXED: Improved octree traversal with proper front-to-back ordering
bool traverse_octree(vec3 ray_origin, vec3 ray_dir, vec3 world_min, vec3 world_max,
                     float t_near, float t_far, out vec3 hit_normal, 
                     out uint hit_voxel, out float hit_t) {
    
    const int MAX_STACK = 23;
    StackEntry stack[MAX_STACK];
    int stack_ptr = 0;
    
    // Start at root
    stack[0].node_idx = 0u;
    stack[0].box_min = world_min;
    stack[0].box_max = world_max;
    stack[0].t_enter = t_near;
    stack_ptr = 1;
    
    int iterations = 0;
    const int MAX_ITERATIONS = 2000;
    
    float closest_hit = t_far;
    
    while (stack_ptr > 0 && iterations < MAX_ITERATIONS) {
        iterations++;
        
        // Pop from stack
        stack_ptr--;
        StackEntry entry = stack[stack_ptr];
        
        // Skip if this node is further than closest hit
        if (entry.t_enter > closest_hit) {
            continue;
        }
        
        // Bounds check
        if (entry.node_idx >= octree_meta.node_count) {
            continue;
        }
        
        OctreeNode node = octree_nodes.nodes[entry.node_idx];
        
        // Check if leaf node
        if (node.child_ptr == 0u) {
            uint voxel_id = node.data & 0xFFu;
            
            if (voxel_id != 0u) {
                // Hit a solid voxel!
                float this_t = max(entry.t_enter, 0.0);
                
                // Only accept if closer than previous hits
                if (this_t < closest_hit) {
                    closest_hit = this_t;
                    hit_t = this_t;
                    hit_voxel = voxel_id;
                    
                    vec3 hit_point = ray_origin + ray_dir * hit_t;
                    
                    // Calculate normal - which face did we enter through?
                    vec3 size = entry.box_max - entry.box_min;
                    vec3 local = (hit_point - entry.box_min) / size;
                    
                    // Determine which face (closest to 0 or 1)
                    const float epsilon = 0.001;
                    vec3 abs_local = abs(local - 0.5);
                    float max_component = max(max(abs_local.x, abs_local.y), abs_local.z);
                    
                    if (abs(abs_local.x - max_component) < epsilon) {
                        hit_normal = vec3(local.x < 0.5 ? -1.0 : 1.0, 0, 0);
                    } else if (abs(abs_local.y - max_component) < epsilon) {
                        hit_normal = vec3(0, local.y < 0.5 ? -1.0 : 1.0, 0);
                    } else {
                        hit_normal = vec3(0, 0, local.z < 0.5 ? -1.0 : 1.0);
                    }
                }
            }
            continue;
        }
        
        // Branch node - subdivide and push children in front-to-back order
        vec3 center = (entry.box_min + entry.box_max) * 0.5;
        
        // Determine ray direction signs for child ordering
        vec3 ray_sign = step(0.0, ray_dir);
        
        // Store children with their t values for sorting
        struct ChildEntry {
            int child_idx;
            float t_near;
            vec3 box_min;
            vec3 box_max;
        };
        
        ChildEntry children[8];
        int child_count = 0;
        
        // Calculate intersections for all 8 children
        for (int i = 0; i < 8; i++) {
            vec3 child_min = vec3(
                (i & 1) != 0 ? center.x : entry.box_min.x,
                (i & 2) != 0 ? center.y : entry.box_min.y,
                (i & 4) != 0 ? center.z : entry.box_min.z
            );
            vec3 child_max = vec3(
                (i & 1) != 0 ? entry.box_max.x : center.x,
                (i & 2) != 0 ? entry.box_max.y : center.y,
                (i & 4) != 0 ? entry.box_max.z : center.z
            );
            
            float child_t_near, child_t_far;
            if (intersect_box(ray_origin, ray_dir, child_min, child_max, 
                            child_t_near, child_t_far)) {
                if (child_t_near < closest_hit) {
                    children[child_count].child_idx = i;
                    children[child_count].t_near = child_t_near;
                    children[child_count].box_min = child_min;
                    children[child_count].box_max = child_max;
                    child_count++;
                }
            }
        }
        
        // Simple insertion sort for front-to-back ordering
        for (int i = 1; i < child_count; i++) {
            ChildEntry key = children[i];
            int j = i - 1;
            while (j >= 0 && children[j].t_near > key.t_near) {
                children[j + 1] = children[j];
                j--;
            }
            children[j + 1] = key;
        }
        
        // Push children onto stack in reverse order (so closest is processed first)
        for (int i = child_count - 1; i >= 0; i--) {
            if (stack_ptr < MAX_STACK) {
                stack[stack_ptr].node_idx = node.child_ptr + uint(children[i].child_idx);
                stack[stack_ptr].box_min = children[i].box_min;
                stack[stack_ptr].box_max = children[i].box_max;
                stack[stack_ptr].t_enter = children[i].t_near;
                stack_ptr++;
            }
        }
    }
    
    return closest_hit < t_far;
}

vec3 calculate_lighting(vec3 normal, vec3 view_dir, vec3 world_pos, vec3 base_color) {
    // Main directional light
    vec3 light_dir = normalize(vec3(0.6, 0.8, 0.5));
    vec3 light_color = vec3(1.0, 0.98, 0.95);
    
    // Diffuse
    float diff = max(dot(normal, light_dir), 0.0);
    
    // Specular
    vec3 reflect_dir = reflect(-light_dir, normal);
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0) * 0.2;
    
    // Ambient with directional variation
    vec3 sky_light = vec3(0.4, 0.5, 0.7);
    float sky_factor = normal.y * 0.5 + 0.5;
    vec3 ambient = mix(vec3(0.2), sky_light * 0.4, sky_factor);
    
    // Combine
    vec3 diffuse = base_color * light_color * diff * 0.6;
    vec3 specular = light_color * spec;
    vec3 ambient_term = base_color * ambient;
    
    return ambient_term + diffuse + specular;
}

void main() {
    vec2 uv = gl_FragCoord.xy / camera.resolution;
    vec3 ray_dir = ray_direction(uv);
    vec3 ray_origin = camera.cam_pos_and_scale.xyz;
    vec3 view_dir = -ray_dir;

    // Use actual octree size from metadata
    float octree_size_f = float(octree_meta.octree_size);
    vec3 box_min = vec3(0.0);
    vec3 box_max = vec3(octree_size_f);
    
    float t_near, t_far;
    
    vec3 sky_color = mix(vec3(0.5, 0.7, 0.9), vec3(0.2, 0.3, 0.5), uv.y);
    
    if (!intersect_box(ray_origin, ray_dir, box_min, box_max, t_near, t_far)) {
        out_color = vec4(sky_color, 1.0);
        return;
    }
    
    vec3 hit_normal;
    uint hit_voxel;
    float hit_t;
    
    if (traverse_octree(ray_origin, ray_dir, box_min, box_max, t_near, t_far,
                        hit_normal, hit_voxel, hit_t)) {
        
        vec3 hit_pos = ray_origin + ray_dir * hit_t;
        vec3 base_color = get_voxel_color(hit_voxel);
        vec3 lit_color = calculate_lighting(hit_normal, view_dir, hit_pos, base_color);
        
        float fog_factor = smoothstep(t_far * 0.6, t_far, hit_t);
        lit_color = mix(lit_color, sky_color * 0.7, fog_factor * 0.3);
        
        out_color = vec4(lit_color, 1.0);
    } else {
        out_color = vec4(sky_color, 1.0);
    }
}