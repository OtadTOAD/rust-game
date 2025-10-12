#version 450

layout(set = 0, binding = 0) uniform usampler3D u_voxel_data;

layout(set = 0, binding = 1) uniform CameraUBO {
    mat4 inv_proj;
    mat4 inv_view;
    vec4 cam_pos_and_scale; // xyz = cam_pos, w = world_scale
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

bool intersect_box(vec3 ray_origin, vec3 ray_dir, vec3 box_min, vec3 box_max, out float t_near, out float t_far) {
    vec3 inv_dir = 1.0 / ray_dir;
    vec3 t_min = (box_min - ray_origin) * inv_dir;
    vec3 t_max = (box_max - ray_origin) * inv_dir;
    
    vec3 t1 = min(t_min, t_max);
    vec3 t2 = max(t_min, t_max);
    
    t_near = max(max(t1.x, t1.y), t1.z);
    t_far = min(min(t2.x, t2.y), t2.z);
    
    // If camera is inside the box, start from 0 (camera position)
    if (t_near < 0.0) {
        t_near = 0.0;
    }
    
    return t_far > t_near;
}

// Get voxel value at integer voxel coordinates
uint get_voxel_at(ivec3 voxel_coord, ivec3 grid_size) {
    if (any(lessThan(voxel_coord, ivec3(0))) || any(greaterThanEqual(voxel_coord, grid_size))) {
        return 0u;
    }
    return texelFetch(u_voxel_data, voxel_coord, 0).r;
}

// Get base color for voxel type
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

// Improved DDA Voxel Traversal
bool dda_raymarch(vec3 ray_origin, vec3 ray_dir, vec3 box_min, vec3 box_max, 
                  float t_near, float t_far, out vec3 hit_normal, out uint hit_voxel, out float hit_t) {
    
    ivec3 grid_size = textureSize(u_voxel_data, 0);
    
    // Start position - handle camera inside volume
    float start_t = max(t_near, 0.0);
    vec3 start_pos = ray_origin + ray_dir * start_t;
    
    // Convert to normalized coordinates [0, 1]
    vec3 norm_pos = (start_pos - box_min) / (box_max - box_min);
    
    // Clamp to valid range with small epsilon to stay inside
    norm_pos = clamp(norm_pos, vec3(0.001), vec3(0.999));
    
    // Convert to voxel coordinates
    vec3 voxel_pos_f = norm_pos * vec3(grid_size);
    ivec3 voxel = ivec3(floor(voxel_pos_f));
    
    // Ensure starting voxel is in bounds
    voxel = clamp(voxel, ivec3(0), grid_size - ivec3(1));
    
    // Step direction
    ivec3 step_dir = ivec3(sign(ray_dir));
    
    // Avoid division by zero
    vec3 safe_ray_dir = ray_dir;
    for (int i = 0; i < 3; i++) {
        if (abs(safe_ray_dir[i]) < 0.00001) {
            safe_ray_dir[i] = 0.00001 * sign(safe_ray_dir[i]);
            if (safe_ray_dir[i] == 0.0) safe_ray_dir[i] = 0.00001;
        }
    }
    
    // Calculate delta_t (how far along ray to move one voxel in each direction)
    vec3 delta_t = abs(vec3(1.0) / safe_ray_dir) / vec3(grid_size);
    
    // Calculate initial t_max (distance to next voxel boundary)
    vec3 t_max;
    for (int i = 0; i < 3; i++) {
        float frac_pos = voxel_pos_f[i] - float(voxel[i]);
        if (step_dir[i] > 0) {
            t_max[i] = (1.0 - frac_pos) * delta_t[i];
        } else {
            t_max[i] = frac_pos * delta_t[i];
        }
    }
    
    // Track which face we hit
    vec3 normal = vec3(0.0);
    
    const int max_steps = 1024;
    float t_current = 0.0;
    float max_t = (t_far - start_t) / length(box_max - box_min);
    
    for (int i = 0; i < max_steps; i++) {
        // Check bounds
        if (any(lessThan(voxel, ivec3(0))) || any(greaterThanEqual(voxel, grid_size))) {
            break;
        }
        
        // Check if exceeded max distance
        if (t_current > max_t) {
            break;
        }
        
        uint voxel_value = get_voxel_at(voxel, grid_size);
        if (voxel_value > 0u) {
            hit_voxel = voxel_value;
            hit_normal = normal;
            hit_t = start_t + t_current * length(box_max - box_min);
            return true;
        }
        
        if (t_max.x < t_max.y) {
            if (t_max.x < t_max.z) {
                t_current = t_max.x;
                t_max.x += delta_t.x;
                voxel.x += step_dir.x;
                normal = vec3(-float(step_dir.x), 0.0, 0.0);
            } else {
                t_current = t_max.z;
                t_max.z += delta_t.z;
                voxel.z += step_dir.z;
                normal = vec3(0.0, 0.0, -float(step_dir.z));
            }
        } else {
            if (t_max.y < t_max.z) {
                t_current = t_max.y;
                t_max.y += delta_t.y;
                voxel.y += step_dir.y;
                normal = vec3(0.0, -float(step_dir.y), 0.0);
            } else {
                t_current = t_max.z;
                t_max.z += delta_t.z;
                voxel.z += step_dir.z;
                normal = vec3(0.0, 0.0, -float(step_dir.z));
            }
        }
    }
    
    return false;
}

// Enhanced lighting
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
    float world_scale = camera.cam_pos_and_scale.w;
    vec3 view_dir = -ray_dir;

    vec3 box_min = vec3(-world_scale * 0.5);
    vec3 box_max = vec3(world_scale * 0.5);
    
    float t_near, t_far;
    
    // Sky gradient
    vec3 sky_color = mix(vec3(0.5, 0.7, 0.9), vec3(0.2, 0.3, 0.5), uv.y);
    
    if (!intersect_box(ray_origin, ray_dir, box_min, box_max, t_near, t_far)) {
        out_color = vec4(sky_color, 1.0);
        return;
    }
    
    // DDA raymarch
    vec3 hit_normal;
    uint hit_voxel;
    float hit_t;
    
    if (dda_raymarch(ray_origin, ray_dir, box_min, box_max, t_near, t_far, 
                     hit_normal, hit_voxel, hit_t)) {
        
        vec3 hit_pos = ray_origin + ray_dir * hit_t;
        vec3 base_color = get_voxel_color(hit_voxel);
        
        // Calculate lighting
        vec3 lit_color = calculate_lighting(hit_normal, view_dir, hit_pos, base_color);
        
        // Distance fog
        float fog_factor = smoothstep(t_far * 0.6, t_far, hit_t);
        lit_color = mix(lit_color, sky_color * 0.7, fog_factor * 0.3);
        
        out_color = vec4(lit_color, 1.0);
    } else {
        out_color = vec4(sky_color, 1.0);
    }
}