#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_albedo_ao;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_surface; // RG = encoded normals, B = roughness, A = metalness

layout(set = 0, binding = 2) uniform sampler2D skybox_hdr;

layout(set = 0, binding = 3) uniform Ambient_Data {
    vec3 color;
    float intensity;
} ambient;

layout(location = 0) out vec4 f_color;

const float PI = 3.14159265359;

vec3 decode_octahedral(vec2 f) {
    f = f * 2.0 - 1.0;
    vec3 n = vec3(f.x, f.y, 1.0 - abs(f.x) - abs(f.y));
    if (n.z < 0.0)
        n.xy = (1.0 - abs(n.yx)) * sign(n.xy);
    return normalize(n);
}

vec2 directionToEquirect(vec3 dir) {
    dir = normalize(dir);
    float phi = atan(dir.z, dir.x);
    float theta = asin(clamp(dir.y, -1.0, 1.0));
    return vec2(phi / (2.0 * PI) + 0.5, theta / PI + 0.5);
}

vec3 sampleSkybox(vec3 dir) {
    return texture(skybox_hdr, directionToEquirect(dir)).rgb;
}

vec3 fresnelSchlickRoughness(float cosTheta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

void main() {
    vec4 albedo_ao = subpassLoad(u_albedo_ao);
    vec4 surface   = subpassLoad(u_surface);

    vec3 albedo     = albedo_ao.rgb;
    float ao        = albedo_ao.a;
    float roughness = clamp(surface.b, 0.04, 1.0);
    float metallic  = surface.a;

    vec3 N = normalize(decode_octahedral(surface.rg));
    
    vec3 V = N;
    vec3 R = reflect(-V, N);
    
    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    
    float NdotV = 0.5;
    vec3 F = fresnelSchlickRoughness(NdotV, F0, roughness);
    
    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);
    
    vec3 irradiance       = sampleSkybox(N);
    vec3 skyUp            = sampleSkybox(vec3(0.0, 1.0, 0.0));
    vec3 skyDown          = sampleSkybox(vec3(0.0, -1.0, 0.0));
    float hemisphereBlend = N.y * 0.5 + 0.5;
    vec3 hemisphere       = mix(skyDown, skyUp, hemisphereBlend);
    irradiance            = mix(hemisphere, irradiance, 0.5);
    
    vec3 diffuse          = kD * albedo * irradiance;
    vec3 prefilteredColor = sampleSkybox(R);
    prefilteredColor      = mix(prefilteredColor, irradiance, roughness * 0.6);
    
    vec3 specular     = prefilteredColor * F;
    vec3 baseAmbient  = albedo * 0.1;
    vec3 ambientLight = (diffuse + specular + baseAmbient) * ao;
    vec3 finalColor   = ambientLight * ambient.color * ambient.intensity;
    f_color           = vec4(finalColor, 1.0);
}