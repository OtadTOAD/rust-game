#version 450

layout(set = 0, binding = 0) uniform sampler3D u_voxel_data;

layout(set = 0, binding = 1) uniform CameraUBO {
    mat4 inv_proj;
    mat4 inv_view;
    vec3 cam_pos;
    float world_scale;
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
    
    return t_far > max(t_near, 0.0);
}

// Calculate normal by sampling nearby voxels
vec3 calculate_normal(vec3 voxel_uv, vec3 box_min, vec3 box_max) {
    float epsilon = 1.0 / camera.world_scale;
    
    float dx_pos = texture(u_voxel_data, voxel_uv + vec3(epsilon, 0, 0)).r;
    float dx_neg = texture(u_voxel_data, voxel_uv - vec3(epsilon, 0, 0)).r;
    float dy_pos = texture(u_voxel_data, voxel_uv + vec3(0, epsilon, 0)).r;
    float dy_neg = texture(u_voxel_data, voxel_uv - vec3(0, epsilon, 0)).r;
    float dz_pos = texture(u_voxel_data, voxel_uv + vec3(0, 0, epsilon)).r;
    float dz_neg = texture(u_voxel_data, voxel_uv - vec3(0, 0, epsilon)).r;
    
    vec3 normal = vec3(
        dx_neg - dx_pos,
        dy_neg - dy_pos,
        dz_neg - dz_pos
    );
    
    return normalize(normal);
}

void main() {
    vec2 uv = gl_FragCoord.xy / vec2(800.0, 600.0); // TODO: Use actual viewport size
    vec3 ray_dir = ray_direction(uv);
    vec3 ray_origin = camera.cam_pos;

    // Define voxel world bounds (centered at origin)
    vec3 box_min = vec3(-camera.world_scale * 0.5);
    vec3 box_max = vec3(camera.world_scale * 0.5);
    
    float t_near, t_far;
    
    // Check if ray intersects the voxel volume
    if(!intersect_box(ray_origin, ray_dir, box_min, box_max, t_near, t_far)) {
        // Background color
        out_color = vec4(0.1, 0.1, 0.2, 1.0);
        return;
    }
    
    // Start raymarching from the entry point
    float t = max(t_near, 0.0);
    vec4 color = vec4(0.0);
    const float dt = 0.05;
    const int max_steps = 256;
    
    for(int i = 0; i < max_steps && t < t_far; i++) {
        vec3 sample_pos = ray_origin + ray_dir * t;
        
        // Transform to voxel texture coordinates [0, 1]
        vec3 voxel_uv = (sample_pos - box_min) / (box_max - box_min);
        
        // Sample the voxel data
        float voxel_value = texture(u_voxel_data, voxel_uv).r;
        
        if(voxel_value > 0.01) {
            // Hit a solid voxel - calculate lighting and stop
            vec3 base_color;
            
            if(voxel_value > 0.5) {
                // Higher value (the sphere)
                base_color = vec3(0.8, 0.2, 0.2);
            } else {
                // Lower value (the floor)
                base_color = vec3(0.2, 0.7, 0.2);
            }
            
            // Calculate normal for lighting
            vec3 normal = calculate_normal(voxel_uv, box_min, box_max);
            
            // Simple directional lighting
            vec3 light_dir = normalize(vec3(0.5, 1.0, 0.3));
            float diffuse = max(dot(normal, light_dir), 0.0);
            
            // Ambient + diffuse lighting
            float ambient = 0.3;
            float lighting = ambient + diffuse * 0.7;
            
            color.rgb = base_color * lighting;
            color.a = 1.0;
            break; // Stop at first solid hit
        }
        
        t += dt;
    }
    
    // Mix with background if nothing was hit
    if(color.a < 0.5) {
        vec3 background = vec3(0.1, 0.1, 0.2);
        color.rgb = background;
        color.a = 1.0;
    }

    out_color = color;
}