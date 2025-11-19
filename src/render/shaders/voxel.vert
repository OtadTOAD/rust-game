#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in mat4 in_model;
layout(location = 5) in mat4 in_model_inv;
layout(location = 9) in mat4 in_model_inv_pose;

layout(location = 0) out vec3 out_world_pos;
layout(location = 1) out mat4 out_model;
layout(location = 5) out mat4 out_model_inv;
layout(location = 9) out mat4 out_model_inv_pose;

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
    vec3 pos;
} cam;

void main() {
    vec4 world_pos = in_model * vec4(in_position, 1.0);
    gl_Position = cam.proj * cam.view * world_pos;

    out_world_pos = world_pos.xyz;
    out_model = in_model;
    out_model_inv = in_model_inv;
    out_model_inv_pose = in_model_inv_pose;
}