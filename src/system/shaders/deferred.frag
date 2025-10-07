#version 450

layout(location = 0) in vec3 in_normal;
layout(location = 1) in vec2 in_tex_coords;
layout(location = 2) in vec3 in_world_pos;
layout(location = 3) in vec4 in_tangent;

layout(set = 1, binding = 1) uniform sampler2D tex_albedo_ao; // RGB = color, A = AO
layout(set = 1, binding = 2) uniform sampler2D tex_material; // RG = encoded normals, B = roughness, A = metalness

layout(location = 0) out vec4 gbuffer_color;
layout(location = 1) out vec4 gbuffer_surface;
layout(location = 2) out vec4 gbuffer_position;

vec3 decode_octahedral(vec2 f) {
    f = f * 2.0 - 1.0;
    vec3 n = vec3(f.x, f.y, 1.0 - abs(f.x) - abs(f.y));
    if (n.z < 0.0)
        n.xy = (1.0 - abs(n.yx)) * sign(n.xy);
    return normalize(n);
}

vec2 encode_octahedral(vec3 n) {
    n /= (abs(n.x) + abs(n.y) + abs(n.z) + 1e-8);
    if (n.z < 0.0)
        n.xy = (1.0 - abs(n.yx)) * sign(n.xy);
    return n.xy * 0.5 + 0.5;
}

void main() {
    vec4 albedo_ao   = texture(tex_albedo_ao, in_tex_coords);
    vec4 mat         = texture(tex_material, in_tex_coords);

    vec3 N = normalize(in_normal);
    vec3 T = normalize(in_tangent.xyz);
    vec3 B = cross(N, T) * in_tangent.w;
    mat3 TBN = mat3(T, B, N);

    vec3 tangentNormal = decode_octahedral(texture(tex_material, in_tex_coords).rg);
    vec3 worldNormal   = normalize(TBN * tangentNormal);
    vec2 encoded       = encode_octahedral(worldNormal);

    gbuffer_color    = albedo_ao;
    gbuffer_surface  = vec4(encoded, mat.b, mat.a);
    gbuffer_position = vec4(in_world_pos, 1.0);
}