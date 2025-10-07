#version 450

layout(location = 0) in vec2 frag_coord;

layout(set = 0, binding = 0) uniform sampler2D hdr_sky;
layout(set = 0, binding = 1) uniform Camera {
    mat4 inv_view;
    mat4 inv_proj;
    vec3 camera_pos;
} camera;

layout(location = 0) out vec4 f_color;

const float PI = 3.14159265359;

vec2 directionToEquirect(vec3 dir) {
    float phi = atan(dir.z, dir.x);
    float theta = asin(dir.y);
    
    vec2 uv;
    uv.x = phi / (2.0 * PI) + 0.5;
    uv.y = theta / PI + 0.5;
    
    return uv;
}

void main() {
    vec4 ndc = vec4(frag_coord * 2.0 - 1.0, -1.0, 1.0); 

    vec4 view = camera.inv_proj * ndc;
    view     /= view.w;

    vec4 world_pos = camera.inv_view * view;
    vec3 ray       = normalize(world_pos.xyz - camera.camera_pos);

    vec2 uv    = directionToEquirect(ray);
    vec3 color = texture(hdr_sky, uv).rgb;
    
    f_color = vec4(color, 1.0);
}