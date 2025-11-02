#version 450

const int MAX_STEPS = 50;
const int CHUNK_SIZE = 32;
const int MAX_CHUNKS = 64;

layout(location = 0) in vec2 fragPosition;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform Camera {
    vec3 cameraPos;
    vec3 cameraForward;
    vec3 cameraRight;
    float fov;
    float aspectRatio;
} camera;


layout(set = 0, binding = 1) buffer VoxelBuffer {
    uint voxels[];
} voxels;

struct ChunkInfo {
    ivec3 chunkPos;
    uint dataOffset;
};
layout(set = 0, binding = 2) buffer ChunkBuffer {
    uint chunkCount;
    ChunkInfo chunks[MAX_CHUNKS];
} chunks;

ivec3 worldToChunk(ivec3 worldPos) {
    return ivec3(floor(vec3(worldPos) / float(CHUNK_SIZE)));
}

ivec3 worldToLocal(ivec3 worldPos) {
    ivec3 local = worldPos % CHUNK_SIZE;
    if (local.x < 0) local.x += CHUNK_SIZE;
    if (local.y < 0) local.y += CHUNK_SIZE;
    if (local.z < 0) local.z += CHUNK_SIZE;
    return local;
}

int findChunk(ivec3 chunkPos) {
    for (int i = 0; i < int(chunks.chunkCount); i++) {
        if (chunks.chunks[i].chunkPos == chunkPos) {
            return i;
        }
    }
    return -1;
}

uint getVoxel(ivec3 worldPos) {
    ivec3 chunkPos = worldToChunk(worldPos);
    int chunkIndex = findChunk(chunkPos);
    
    if (chunkIndex == -1) {
        return 0;
    }
    
    ivec3 localPos = worldToLocal(worldPos);
    uint voxelIndex = uint(localPos.z + localPos.y * CHUNK_SIZE + localPos.x * CHUNK_SIZE * CHUNK_SIZE);
    uint dataOffset = chunks.chunks[chunkIndex].dataOffset;
    return voxels.voxels[dataOffset + voxelIndex];
}

struct RayHit {
    bool hit;
    vec3 position;
    vec3 normal;
    uint voxelValue;
};

RayHit raycastVoxel(vec3 origin, vec3 rayDir) {
    RayHit result;
    result.hit = false;

    vec3 safeDelta = rayDir;
    for (int i = 0; i < 3; i++) {
        if (abs(rayDir[i]) < 0.0001) {
            safeDelta[i] = 0.0001;
        }
    }

    // Start from the camera position
    ivec3 voxelPos = ivec3(floor(origin));
    ivec3 step = ivec3(sign(safeDelta));
    vec3 tDelta = abs(vec3(1.0) / safeDelta);

    // Calculate initial tMax values
    vec3 tMax;
    for (int i = 0; i < 3; i++) {
        if (step[i] > 0) {
            tMax[i] = (float(voxelPos[i] + 1) - origin[i]) * tDelta[i];
        } else {
            tMax[i] = (origin[i] - float(voxelPos[i])) * tDelta[i];
        }
    }

    ivec3 normal = ivec3(0, 1, 0); // Default normal
    
    // DDA traversal through world space
    for (int i = 0; i < MAX_STEPS; i++) {
        uint voxelValue = getVoxel(voxelPos);
        if (voxelValue != 0) {
            result.hit = true;
            result.position = vec3(voxelPos);
            result.normal = vec3(normal); 
            result.voxelValue = voxelValue;
            return result;
        }

        if (tMax.x < tMax.y) {
            if (tMax.x < tMax.z) {
                voxelPos.x += step.x;
                tMax.x += tDelta.x;
                normal = ivec3(-step.x, 0, 0);
            } else {
                voxelPos.z += step.z;
                tMax.z += tDelta.z;
                normal = ivec3(0, 0, -step.z);
            }
        } else {
            if (tMax.y < tMax.z) {
                voxelPos.y += step.y;
                tMax.y += tDelta.y;
                normal = ivec3(0, -step.y, 0);
            } else {
                voxelPos.z += step.z;
                tMax.z += tDelta.z;
                normal = ivec3(0, 0, -step.z);
            }
        }
    }

    return result;
}

vec3 chunkColor(ivec3 chunkPos) {
    // Hash the chunk position to get a unique color
    int hash = chunkPos.x * 73856093 ^ chunkPos.y * 19349663 ^ chunkPos.z * 83492791;
    hash = abs(hash);
    
    float r = float((hash >> 0) & 255) / 255.0;
    float g = float((hash >> 8) & 255) / 255.0;
    float b = float((hash >> 16) & 255) / 255.0;
    
    // Normalize and add some brightness
    vec3 color = vec3(r, g, b);
    color = normalize(color) * 0.7 + 0.3;
    
    return color;
}


void main() {    
    vec3 up = cross(camera.cameraForward, camera.cameraRight);
    
    vec3 rayDir = normalize(
        camera.cameraRight * fragPosition.x * camera.aspectRatio * tan(camera.fov * 0.5) +
        up * fragPosition.y * tan(camera.fov * 0.5) +
        camera.cameraForward
    );

    RayHit hit = raycastVoxel(camera.cameraPos, rayDir);
    if (hit.hit) {
        ivec3 chunkPos = worldToChunk(ivec3(hit.position));
        vec3 chunkCol = chunkColor(chunkPos);

        vec3 lightDir = normalize(vec3(1.0, 2.0, 1.0));
        float diffuse = max(dot(hit.normal, lightDir) * 0.5 + 0.5, 0.0);
        vec3 color = chunkCol * (0.3 + 0.7 * diffuse);
        outColor = vec4(color, 1.0);
    } else {
        outColor = vec4(0.5, 0.7, 1.0, 1.0);
    }
}