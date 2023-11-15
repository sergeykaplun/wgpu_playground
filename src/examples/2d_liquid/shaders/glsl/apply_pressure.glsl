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

vec2 calc_pressure_force(uint particle_index) {
    vec2 pressure_force = vec2(0.0);

    Particle cur_particle = particle_data[particle_index];
    ivec2 center = PARTICLE_CELL(cur_particle.predicted_pos);
    //ivec2 center = PARTICLE_CELL(cur_particle.pos);
    for(int i = -1; i <= 1; i++) {
        for(int j = -1; j <= 1; j++) {
            //if(any(cell_coord < vec2(0)) && any(cell_coord >= vec2<i32>(ceil(constants.bounds_size / constants.smoothing_radius)))) {
            //    continue;
            //}
            ivec2 cell_coord = center + ivec2(i, j);
            uint hash = CELL_HASH(cell_coord);
            uint key = CELL_KEY(hash);

            uint cellStartIndex = start_indices[key];
            for (uint i=cellStartIndex; i<constants.particles_count; i++) {
                if(spatial_lookup[i].cell_key != key)  break;
                uint other_particle_index = spatial_lookup[i].particle_id;
                if (particle_index == other_particle_index) { continue; }
                Particle other_particle = particle_data[other_particle_index];
                vec2 offset = other_particle.predicted_pos - cur_particle.predicted_pos;
                //vec2 offset = other_particle.pos - cur_particle.predicted_pos;
                float dist = length(offset);
                vec2 dir = normalize(offset);   //TODO is NaN occurring here?
                if(dist <= constants.smoothing_radius) {
                    float slope = smooth_kernel_derivative(dist, constants.smoothing_radius);
                    float density = other_particle.density;
                    float shared_pressure = (DENS_2_PRESS(density, constants.target_density, constants.pressure_multiplier)
                                           + DENS_2_PRESS(cur_particle.density, constants.target_density, constants.pressure_multiplier)) * 0.5;
                    float shared_pressure = (DENS_2_PRESS(density, p, constants.pressure_multiplier)
                                           + DENS_2_PRESS(cur_particle.density, p, constants.pressure_multiplier)) * 0.5;
                    pressure_force += shared_pressure * dir * slope * constants.particle_mass / density;
                }
            }
        }
    }
    return pressure_force;
}

/*
vec2 calc_pressure_force(uint particle_index) {
    vec2 pressure_force = vec2(0.0);
    Particle cur_particle = particle_data[particle_index];
    for (uint other_particle_index = 0; other_particle_index < constants.particles_count; other_particle_index++) {
        if (particle_index == other_particle_index) { continue; }
        Particle other_particle = particle_data[other_particle_index];
        vec2 offset = other_particle.predicted_pos - cur_particle.predicted_pos;
        float dist = length(offset);
        vec2 dir = normalize(offset);   //TODO is NaN occurring here?

        float slope = smooth_kernel_derivative(dist, constants.smoothing_radius);
        float density = other_particle.density;
        float shared_pressure = (DENS_2_PRESS(density, constants.target_density, constants.pressure_multiplier) + DENS_2_PRESS(cur_particle.density, constants.target_density, constants.pressure_multiplier)) * 0.5;
        pressure_force += shared_pressure * dir * slope * constants.particle_mass / density;
    }

    return pressure_force;
}
*/


layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    vec2 pressure_force = calc_pressure_force(gl_GlobalInvocationID.x);
    vec2 pressure_accel = pressure_force / particle.density;
    particle.vel += pressure_accel * constants.delta_time;
    particle_data[gl_GlobalInvocationID.x] = particle;
}
