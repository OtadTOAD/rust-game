#version 450

layout(location = 0) in vec2 fragPosition;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) buffer VoxelBuffer {
    uint voxels[];
} voxels;

layout(set = 0, binding = 1) uniform Camera {
    vec3 cameraPos;
    vec3 cameraForward;
    vec3 cameraRight;
    float fov;
} camera;

uint getVoxel(vec3 pos) {
    ivec3 voxelCoord = ivec3(floor(pos));
    if (voxelCoord.x < 0 || voxelCoord.y < 0 || voxelCoord.z < 0) {
        return 0;
    }
    if (voxelCoord.x >= 16 || voxelCoord.y >= 16 || voxelCoord.z >= 16) {
        return 0;
    }

    uint voxelIndex = uint(voxelCoord.z + voxelCoord.y * 16 + voxelCoord.x * 16 * 16);
    return voxels.voxels[voxelIndex];
}

const int MAX_STEPS = 500;
const float stepSize = 0.1;

void main() {
    float aspectRatio = 800.0 / 600.0; // Assume fixed resolution for simplicity
    vec3 up = cross(normalize(camera.cameraForward), normalize(camera.cameraRight));
    vec3 rayDir = normalize(
        camera.cameraRight * fragPosition.x * aspectRatio * tan(camera.fov * 0.5) +
        up * fragPosition.y * tan(camera.fov * 0.5) +
        camera.cameraForward
    );

    vec3 pos = camera.cameraPos;
    for (int i = 0; i < MAX_STEPS; i++) {
        if (getVoxel(pos) != 0) {
            // Debug: Show which voxel coordinate we hit
            ivec3 voxelCoord = ivec3(floor(pos));
            outColor = vec4(
                float(voxelCoord.x) / 16.0,  // Red = X position
                float(voxelCoord.y) / 16.0,  // Green = Y position
                float(voxelCoord.z) / 16.0,  // Blue = Z position
                1.0
            );
            return;
        }
        pos += rayDir * stepSize;
    }

    outColor = vec4(fragPosition, 0.0, 1.0);
}