#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_albedo_ao;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_surface; // RG = encoded normals, B = roughness, A = metalness
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput u_frag_position;

layout(set = 0, binding = 3) uniform sampler2D skybox_hdr;

layout(set = 0, binding = 4) uniform Ambient_Data {
    vec3 color;
    float intensity;
} ambient;

layout(set = 0, binding = 5) uniform Camera_Data {
    vec3 camera_pos;
} camera;

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

vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

vec3 fresnelSchlickRoughness(float cosTheta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

vec3 sampleSkyboxRoughness(vec3 dir, float roughness) {
    vec3 color = vec3(0.0);
    float totalWeight = 0.0;
    
    int samples = int(mix(1.0, 8.0, roughness));
    
    for (int i = 0; i < samples; i++) {
        float angle = float(i) * 2.0 * PI / float(samples);
        float spread = roughness * 0.3;
        
        vec3 tangent = normalize(cross(dir, vec3(0.0, 1.0, 0.0)));
        if (length(tangent) < 0.1) {
            tangent = normalize(cross(dir, vec3(1.0, 0.0, 0.0)));
        }
        vec3 bitangent = cross(dir, tangent);
        
        vec3 offset = tangent * cos(angle) * spread + bitangent * sin(angle) * spread;
        vec3 sampleDir = normalize(dir + offset);
        
        color += sampleSkybox(sampleDir);
        totalWeight += 1.0;
    }
    
    return color / totalWeight;
}

void main() {
    vec4 albedo_ao = subpassLoad(u_albedo_ao);
    vec4 surface   = subpassLoad(u_surface);
    vec3 fragPos   = subpassLoad(u_frag_position).xyz;
    vec3 camPos    = camera.camera_pos;

    // Otherwise it kinda affects background too
    if (length(fragPos) > 1000.0 || (albedo_ao.r == 0.0 && albedo_ao.g == 0.0 && albedo_ao.b == 0.0)) {
        f_color = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec3  albedo    = albedo_ao.rgb;
    float ao        = albedo_ao.a;
    float roughness = surface.b;
    float metallic  = surface.a;
    vec3  normal    = decode_octahedral(surface.rg);
    
    vec3  V     = normalize(camPos - fragPos);
    vec3  F0    = mix(vec3(0.04), albedo, metallic);
    float NdotV = max(dot(normal, V), 0.0);
    vec3  F     = fresnelSchlickRoughness(NdotV, F0, roughness);
    
    
    vec3 R                  = reflect(-V, normal);
    vec3 specularIrradiance = sampleSkyboxRoughness(R, roughness * 0.5);

    vec3 irradiance = sampleSkyboxRoughness(normal, roughness);
    vec3 kD         = (1.0 - F) * (1.0 - metallic);
    vec3 diffuse    = kD * albedo * irradiance;
    vec3 specular   = specularIrradiance * F;
    
    vec3 ambientColor = ambient.color * ambient.intensity;
    
    vec3 finalColor = (diffuse + specular) * ambientColor * ao;
    finalColor     += irradiance * ambientColor * 0.1 * ao;
    f_color         = vec4(finalColor, 1.0);
}