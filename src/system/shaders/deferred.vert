#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;
layout(location = 3) in vec2 uv;
layout(location = 4) in mat4 instance_model;
layout(location = 8) in mat4 instance_normal;// Skips to 8 because mat4 takes up 4 spots

layout(location = 0) out vec3 out_color;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_location;
layout(location = 3) out vec2 out_tex_coords;

layout(set = 0, binding = 0) uniform VP_Data {
    mat4 view;
    mat4 projection;
} vp_uniforms;

void main() {
    vec4 frag_pos = vp_uniforms.projection * vp_uniforms.view * instance_model * vec4(position, 1.0);
    gl_Position = frag_pos;
    out_color = color;
    out_normal = mat3(instance_normal) * normal;
    out_location = frag_pos;
    out_tex_coords = uv;
}