#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_albedo_ao; // RGB = color, A = AO
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_surface; // RG = encoded normals, B = roughness, A = metalness
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_frag_position;

layout(set = 0, binding = 4) uniform Directional_Light_Data {
    vec4 position;  // xyz = direction
    vec3 color;
} light;

layout(set = 0, binding = 5) uniform Camera_Data {
    vec3 position;
} camera;

layout(location = 0) out vec4 f_color;

vec3 decode_octahedral(vec2 f) {
    f = f * 2.0 - 1.0;
    vec3 n = vec3(f.x, f.y, 1.0 - abs(f.x) - abs(f.y));
    if (n.z < 0.0)
        n.xy = (1.0 - abs(n.yx)) * sign(n.xy);
    return normalize(n);
}

void main() {
    vec4 albedoAO = subpassLoad(u_albedo_ao);
    vec4 surface  = subpassLoad(u_surface);
    vec3 fragPos  = subpassLoad(u_frag_position).xyz;

    vec3 albedo = albedoAO.rgb;
    float ao    = albedoAO.a;

    vec2 encN       = surface.rg;
    float roughness = surface.b;
    float metallic  = surface.a;

    vec3 normal = decode_octahedral(encN);

    vec3 lightDir = normalize(light.position.xyz - fragPos);
    vec3 viewDir  = normalize(camera.position - fragPos);

    float ndl    = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = albedo * light.color * ndl;

    vec3 halfway  = normalize(lightDir + viewDir);
    float ndh     = max(dot(normal, halfway), 0.0);
    float spec    = pow(ndh, 1.0 / max(roughness, 0.001));
    vec3 specular = spec * mix(vec3(0.04), albedo, metallic);

    vec3 color = (diffuse + specular) * ao;
    f_color    = vec4(color, 1.0);
}