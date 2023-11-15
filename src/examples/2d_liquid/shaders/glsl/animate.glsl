#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};

#define SPEED 3.5
layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    vec2 off = particle.target_pos - particle.pos;
    if(length(off) > 0.05)
        particle.vel += normalize(off) * constants.animate_strength * constants.delta_time;
    particle_data[gl_GlobalInvocationID.x] = particle;
}
