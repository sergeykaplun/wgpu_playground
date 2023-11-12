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
layout(push_constant) uniform SortingParamsData {
    SortingParams sorting_params;
} pushConstants;

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    uint i = gl_GlobalInvocationID.x;

    uint h = i & (pushConstants.sorting_params.group_width - 1u);
    uint index_low = h + (pushConstants.sorting_params.group_height + 1u) * (i / pushConstants.sorting_params.group_width);
    uint index_high = index_low + (pushConstants.sorting_params.step_index == 0u ? pushConstants.sorting_params.group_height - 2u * h : (pushConstants.sorting_params.group_height + 1u) / 2u);

    if (index_high >= constants.particles_count) {
        return;
    }

    SpatialLookupItem value_low = spatial_lookup[index_low];
    SpatialLookupItem value_high = spatial_lookup[index_high];

    if (value_low.cell_key > value_high.cell_key) {
        spatial_lookup[index_low] = value_high;
        spatial_lookup[index_high] = value_low;
    }
}
