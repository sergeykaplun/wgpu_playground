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

vec2 calc_viscosity_force(uint particleIndex) {
    vec2 viscosityForce = vec2(0.0);
    Particle particle = particle_data[particleIndex];
    ivec2 center = PARTICLE_CELL(particle.pos);
    for(int i = -1; i <= 1; i++) {
        for(int j = -1; j <= 1; j++) {
            ivec2 cell_coord = center + ivec2(i, j);
            //TODO: check if cell is out of bounds
            uint hash = CELL_HASH(cell_coord);
            uint key = CELL_KEY(hash);

            uint cellStartIndex = start_indices[key];
            for (uint i=cellStartIndex; i<constants.particles_count; i++) {
                if(spatial_lookup[i].cell_key != key)  break;
                uint other_particle_index = spatial_lookup[i].particle_id;
                Particle other_particle = particle_data[other_particle_index];
                //TODO pos instead of predicted_pos???
                float dst = distance(other_particle.pos, particle.pos);
                float influence = viscosity_smooth_kernel(dst, constants.smoothing_radius);
                viscosityForce += (other_particle.vel - particle.vel) * influence;
            }
        }
    }
    return viscosityForce * constants.viscosity;
}

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    particle.vel += calc_viscosity_force(gl_GlobalInvocationID.x) * constants.delta_time;
    particle_data[gl_GlobalInvocationID.x] = particle;
}
