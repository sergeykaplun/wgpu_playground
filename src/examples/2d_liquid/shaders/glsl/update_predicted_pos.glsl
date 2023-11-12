#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    //particle.vel += /*constants.gravity_strength * */constants.gravity * constants.delta_time;
    particle.predicted_pos = particle.pos + particle.vel * 1./120.;
    particle_data[gl_GlobalInvocationID.x] = particle;
}
