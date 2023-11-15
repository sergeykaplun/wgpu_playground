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

float calc_density(vec2 sample_point) {
    float density = 0.0;
    ivec2 center = PARTICLE_CELL(sample_point);
    for(int i = -1; i <= 1; i++) {
        for(int j = -1; j <= 1; j++) {
            ivec2 cell_coord = center + ivec2(i, j);
            //TODO: check if cell is out of bounds
            uint hash = CELL_HASH(cell_coord);
            uint key = CELL_KEY(hash);

            uint cellStartIndex = start_indices[key];
            for (uint i=cellStartIndex; i<constants.particles_count; i++) {
                if(spatial_lookup[i].cell_key != key)  break;
                uint particle_index = spatial_lookup[i].particle_id;
                Particle particle = particle_data[particle_index];
                //TODO pos instead of predicted_pos
                float dst = distance(particle.pos, sample_point);
                if(dst <= constants.smoothing_radius) {
                    float influence = smooth_kernel(dst, constants.smoothing_radius);
                    density += influence;
                }
            }
        }
    }
    return density * constants.particle_mass;
}

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    //vec2 sample_point, Particle[] particles, uint particles_count, float smoothing_radius, float particle_mass
    particle.density = calc_density(particle.predicted_pos);
    particle_data[gl_GlobalInvocationID.x] = particle;
}
