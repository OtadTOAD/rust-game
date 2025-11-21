#version 450

layout(location = 0) in vec3 in_world_pos;
layout(location = 1) in mat4 in_model;
layout(location = 5) in mat4 in_model_inv;
layout(location = 9) in mat4 in_model_inv_pose;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec4 out_albedo;
layout(location = 2) out vec4 out_normal;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
    vec3 pos;
} cam;

layout(set = 1, binding = 0) uniform usampler3D voxel_texture;

// To keep this simple AABB, we assume ro and rd are in local space.
// Ray gets transformed into local space before calling this.
const float EPS = 1e-8;
bool intersectAABB(vec3 ro, vec3 rd, vec3 min_b, vec3 max_b, out float t0, out float t1) {
    vec3 inv_dir = vec3(
        abs(rd.x) > EPS ? 1.0 / rd.x : 1e30,
        abs(rd.y) > EPS ? 1.0 / rd.y : 1e30,
        abs(rd.z) > EPS ? 1.0 / rd.z : 1e30
    );

    vec3 t0s = (min_b - ro) * inv_dir;
    vec3 t1s = (max_b - ro) * inv_dir;

    vec3 tminv = min(t0s, t1s);
    vec3 tmaxv = max(t0s, t1s);

    t0 = max(max(tminv.x, tminv.y), tminv.z);
    t1 = min(min(tmaxv.x, tmaxv.y), tmaxv.z);
    return t1 >= max(t0, 0.0);
}

// A Fast Voxel Traversal Algorithm for Ray Tracing by John Amanatides & Andrew Woo
// https://www.cs.yorku.ca/~amana/research/grid.pdf
bool voxelDDA(vec3 ro, vec3 rd, vec3 min_b, vec3 max_b, ivec3 grid_size, out ivec3 hit_voxel, out float t_hit, out vec3 normal) {
    float t_in, t_out;
    if (!intersectAABB(ro, rd, min_b, max_b, t_in, t_out)) {
        return false;
    }

    t_in = max(t_in, 0.0);
    vec3 pos = ro + rd * t_in;
    vec3 pos_in_grid = pos;
    ivec3 voxel = ivec3(floor(pos_in_grid));
    voxel = clamp(voxel, ivec3(0), grid_size - ivec3(1));

    vec3 hit_normal = vec3(0.0);
    if (t_in > 0.0) {
        vec3 hit_point = ro + rd * t_in;
        vec3 center = (min_b + max_b) * 0.5;
        vec3 to_hit = hit_point - center;
        vec3 half_size = (max_b - min_b) * 0.5;
        vec3 abs_to_hit = abs(to_hit);
        
        if (abs_to_hit.x / half_size.x > abs_to_hit.y / half_size.y && 
            abs_to_hit.x / half_size.x > abs_to_hit.z / half_size.z) {
            hit_normal = vec3(sign(to_hit.x), 0.0, 0.0);
        } else if (abs_to_hit.y / half_size.y > abs_to_hit.z / half_size.z) {
            hit_normal = vec3(0.0, sign(to_hit.y), 0.0);
        } else {
            hit_normal = vec3(0.0, 0.0, sign(to_hit.z));
        }
    }

    ivec3 step;
    vec3 t_max;
    vec3 t_delta;
    for (int i = 0; i < 3; i++) {
        if (rd[i] > 0.0) {
            step[i] = 1;
            t_max[i] = t_in + ((float(voxel[i]) + 1.0) - pos_in_grid[i]) / rd[i];
            t_delta[i] = 1.0 / rd[i];
        } else if (rd[i] < 0.0) {
            step[i] = -1;
            t_max[i] = t_in + (float(voxel[i]) - pos_in_grid[i]) / rd[i];
            t_delta[i] = -1.0 / rd[i];
        } else {
            step[i] = 0;
            t_max[i] = 1e30;
            t_delta[i] = 1e30;
        }
    }

    float t = t_in;
    
    while (t <= t_out) {
        
        if (voxel.x < 0 || voxel.y < 0 || voxel.z < 0 ||
            voxel.x >= grid_size.x || voxel.y >= grid_size.y || voxel.z >= grid_size.z) {
            break;
        }
        
        uint v = texelFetch(voxel_texture, voxel, 0).r;
        if (v != 0u) {
            hit_voxel = voxel;
            t_hit = t;
            normal = hit_normal;
            return true;
        }

        if (t_max.x < t_max.y) {
            if (t_max.x < t_max.z) {
                voxel.x += step.x;
                t = t_max.x;
                t_max.x += t_delta.x;
                hit_normal = vec3(-step.x, 0.0, 0.0);
            } else {
                voxel.z += step.z;
                t = t_max.z;
                t_max.z += t_delta.z;
                hit_normal = vec3(0.0, 0.0, -step.z);
            }
        } else {
            if (t_max.y < t_max.z) {
                voxel.y += step.y;
                t = t_max.y;
                t_max.y += t_delta.y;
                hit_normal = vec3(0.0, -step.y, 0.0);
            } else {
                voxel.z += step.z;
                t = t_max.z;
                t_max.z += t_delta.z;
                hit_normal = vec3(0.0, 0.0, -step.z);
            }
        }
    }
    return false;
}

void main() {
    vec3 rd_world = normalize(in_world_pos - cam.pos);

    ivec3 grid_size = textureSize(voxel_texture, 0);

    vec3 ro_unit = vec3(in_model_inv * vec4(cam.pos, 1.0));
    vec3 ro_local = (ro_unit + vec3(0.5)) * vec3(grid_size);
    
    vec3 in_pos_unit = vec3(in_model_inv * vec4(in_world_pos, 1.0));
    vec3 in_pos_local = (in_pos_unit + vec3(0.5)) * vec3(grid_size);
    vec3 rd_local = normalize(in_pos_local - ro_local);

    vec3 box_min = vec3(0.0);
    vec3 box_max = vec3(grid_size);
    
    ivec3 hit;
    float t_hit;
    vec3 normal_local;
    if (!voxelDDA(ro_local, rd_local, box_min, box_max, grid_size, hit, t_hit, normal_local)) {
        discard;
    }

    // We need to manually write to depth buffer since voxels don't generally fill out
    // the entire bounding box, and we can't know depth before DDA.
    // Not most optimal, since this will only cull fragments after this shader stage,
    // but works for now. Since we at least skip lighting and output for missed rays,
    // it should still be a win.
    vec3 hit_pos_local = ro_local + rd_local * t_hit;
    vec3 hit_pos_unit = (hit_pos_local / vec3(grid_size)) - vec3(0.5);
    vec3 hit_pos_world = vec3(in_model * vec4(hit_pos_unit, 1.0));

    vec4 clip_pos = cam.proj * cam.view * vec4(hit_pos_world, 1.0);
    gl_FragDepth = (clip_pos.z / clip_pos.w) * 0.5 + 0.5;

    vec3 normal_world = normalize(mat3(in_model_inv_pose) * normal_local);

    // TODO: Use voxel idx to look up material in pallette
    vec3 albedo = vec3(1.0, 0.0, 0.0);
    
    // Basic directional lighting for testing
    vec3 light_dir = normalize(vec3(0.5, -0.5, 0.5));
    float diffuse = max(dot(normal_world, light_dir), 0.0);
    float ambient = 0.5;
    vec3 axis_factor = abs(normal_world);
    float axis_variation = axis_factor.x * 0.95 + axis_factor.y * 1.0 + axis_factor.z * 0.90;
    float lighting = (ambient + diffuse * (1.0 - ambient)) * axis_variation;
    
    // G-buffer
    out_albedo = vec4(albedo, 1.0);
    out_normal = vec4(normal_world * 0.5 + 0.5, 1.0); 
    out_color = vec4(albedo * lighting, 1.0);
}