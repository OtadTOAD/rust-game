#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 tangent;
layout(location = 3) in vec2 uv;
layout(location = 4) in mat4 instance_model;
layout(location = 8) in mat4 instance_normal;

layout(location = 0) out vec3 out_normal;
layout(location = 1) out vec2 out_tex_coords;
layout(location = 2) out vec3 out_world_pos;
layout(location = 3) out vec4 out_tangent;

layout(set = 0, binding = 0) uniform VP_Data {
    mat4 view;
    mat4 projection;
} vp_uniforms;

void main() {
    vec4 world_pos = instance_model * vec4(position, 1.0);
    gl_Position    = vp_uniforms.projection * vp_uniforms.view * world_pos;
    
    out_normal     = normalize(mat3(instance_normal) * normal);
    out_tangent    = vec4(normalize(mat3(instance_normal) * tangent.xyz), tangent.w);
    out_tex_coords = uv;
    out_world_pos  = world_pos.xyz;
}