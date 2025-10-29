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
    float aspectRatio;
} camera;

uint getVoxel(ivec3 voxelCoord) {
    if (voxelCoord.x < 0 || voxelCoord.y < 0 || voxelCoord.z < 0) {
        return 0;
    }
    if (voxelCoord.x >= 16 || voxelCoord.y >= 16 || voxelCoord.z >= 16) {
        return 0;
    }

    uint voxelIndex = uint(voxelCoord.z + voxelCoord.y * 16 + voxelCoord.x * 16 * 16);
    return voxels.voxels[voxelIndex];
}

struct RayHit {
    bool hit;
    vec3 position;
    vec3 normal;
    uint voxelValue;
};

const int MAX_STEPS = 200;

// Ray-AABB intersection to find entry point into voxel grid(IDK ask claude)
bool rayBoxIntersection(vec3 origin, vec3 rayDir, vec3 boxMin, vec3 boxMax, out float tNear, out float tFar) {
    vec3 invDir = 1.0 / rayDir;
    vec3 t0 = (boxMin - origin) * invDir;
    vec3 t1 = (boxMax - origin) * invDir;
    
    vec3 tmin = min(t0, t1);
    vec3 tmax = max(t0, t1);
    
    tNear = max(max(tmin.x, tmin.y), tmin.z);
    tFar = min(min(tmax.x, tmax.y), tmax.z);
    
    return tNear <= tFar && tFar > 0.0;
}

RayHit raycastVoxel(vec3 origin, vec3 rayDir) {
    RayHit result;
    result.hit = false;

    vec3 safeDelta = rayDir;
    for (int i = 0; i < 3; i++) {
        if (abs(rayDir[i]) < 0.0001) {
            safeDelta[i] = 0.0001;
        }
    }

    // Find entry point into the grid
    float tNear, tFar;
    vec3 gridMin = vec3(0.0);
    vec3 gridMax = vec3(16.0);
    
    if (!rayBoxIntersection(origin, safeDelta, gridMin, gridMax, tNear, tFar)) {
        return result; // Ray doesn't hit the grid at all
    }
    
    // Start from entry point (or origin if inside)
    vec3 startPos = origin;
    if (tNear > 0.0) {
        startPos = origin + safeDelta * (tNear + 0.001); // Small epsilon to ensure we're inside
    }

    ivec3 voxelPos = ivec3(floor(startPos));
    ivec3 step = ivec3(sign(safeDelta));
    vec3 tDelta = abs(vec3(1.0) / safeDelta);

    vec3 tMax;
    for (int i = 0; i < 3; i++) {
        if (step[i] > 0) {
            tMax[i] = (float(voxelPos[i] + 1) - startPos[i]) * tDelta[i];
        } else {
            tMax[i] = (startPos[i] - float(voxelPos[i])) * tDelta[i];
        }
    }

    // Initialize normal based on entry face
    ivec3 normal = ivec3(0);
    if (tNear > 0.0) {
        // Ray entered from outside - determine which face
        vec3 entryPoint = origin + safeDelta * tNear;
        vec3 epsilon = vec3(0.001);
        
        if (abs(entryPoint.x - gridMin.x) < epsilon.x) normal = ivec3(-1, 0, 0);
        else if (abs(entryPoint.x - gridMax.x) < epsilon.x) normal = ivec3(1, 0, 0);
        else if (abs(entryPoint.y - gridMin.y) < epsilon.y) normal = ivec3(0, -1, 0);
        else if (abs(entryPoint.y - gridMax.y) < epsilon.y) normal = ivec3(0, 1, 0);
        else if (abs(entryPoint.z - gridMin.z) < epsilon.z) normal = ivec3(0, 0, -1);
        else if (abs(entryPoint.z - gridMax.z) < epsilon.z) normal = ivec3(0, 0, 1);
    }
    
    for (int i = 0; i < MAX_STEPS; i++) {
        if (voxelPos.x < 0 || voxelPos.x >= 16 ||
            voxelPos.y < 0 || voxelPos.y >= 16 ||
            voxelPos.z < 0 || voxelPos.z >= 16) {
            break;
        }

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

void main() {    
    vec3 up = cross(camera.cameraForward, camera.cameraRight);
    
    vec3 rayDir = normalize(
        camera.cameraRight * fragPosition.x * camera.aspectRatio * tan(camera.fov * 0.5) +
        up * fragPosition.y * tan(camera.fov * 0.5) +
        camera.cameraForward
    );

    RayHit hit = raycastVoxel(camera.cameraPos, rayDir);
    if (hit.hit) {
        vec3 lightDir = normalize(vec3(1.0, 2.0, 1.0));
        float diffuse = max(dot(hit.normal, lightDir) * 0.5 + 0.5, 0.0);
        vec3 color = vec3(0.8, 0.8, 0.8) * (0.3 + 0.7 * diffuse);
        outColor = vec4(color, 1.0);
    } else {
        outColor = vec4(0.5, 0.7, 1.0, 1.0);
    }
}