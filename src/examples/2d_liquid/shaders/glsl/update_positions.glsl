#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};

vec2 pointer_location() {
    return (constants.pointer_location/constants.resolution * 2.0 - 1.0) * 5.0 * vec2(constants.aspect, -1.0);
}

vec2 calculate_interaction_force(vec2 pos, vec2 vel) {
    vec2 force = vec2(0.0);
    vec2 offset = pointer_location() - pos;
    if (constants.pointer_attract > 0.0) {
        offset *= -1.0;
    }
    float dist = length(offset);
    float radius = 3.0;
    float strength = constants.pressure_multiplier * 2.0;
    if(dist < radius) {
        vec2 dir_to_pointer = normalize(offset);
        float cntr = 1.0 - dist / radius;
        force += (dir_to_pointer * strength - vel) * cntr;
    }

    return force;
}

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];

    if (constants.pointer_active == 1.0) {
        particle.vel += calculate_interaction_force(particle.pos, particle.vel) * constants.delta_time;
    }
    particle.pos += particle.vel * constants.delta_time;
    vec2 half_bounds = constants.bounds_size * 0.5 - constants.particle_radius;
    if (abs(particle.pos.x) > half_bounds.x) {
        particle.pos.x = sign(particle.pos.x) * half_bounds.x;
        particle.vel.x *= -constants.damping;
    }
    if (abs(particle.pos.y) > half_bounds.y) {
        particle.pos.y = sign(particle.pos.y) * half_bounds.y;
        particle.vel.y *= -constants.damping;
    }

    particle_data[gl_GlobalInvocationID.x] = particle;
}
