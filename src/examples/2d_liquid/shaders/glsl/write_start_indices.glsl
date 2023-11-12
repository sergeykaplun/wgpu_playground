#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};
layout(set = 1, binding = 1) buffer SpatialLookupData {
    SpatialLookupItem spatial_lookup[];
};
layout(set = 1, binding = 2) buffer StartindicesData {
    uint start_indices[];
};

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    uint id = gl_GlobalInvocationID.x;
    uint key = spatial_lookup[id].cell_key;
    //TODO its probably error here
    uint keyPrev = id == 0u ? MAX_UINT : spatial_lookup[id - 1u].cell_key;
    if (key != keyPrev) {
        start_indices[key] = id;
    }
}
