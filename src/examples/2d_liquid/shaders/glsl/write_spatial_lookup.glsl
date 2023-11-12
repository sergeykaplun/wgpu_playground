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
    Particle particle = particle_data[id];
    ivec2 cell_coord = PARTICLE_CELL(particle.predicted_pos);
    uint cell_hash = CELL_HASH(cell_coord);

    spatial_lookup[id] = SpatialLookupItem(id, CELL_KEY(cell_hash));
    start_indices[id] = MAX_UINT;
}
