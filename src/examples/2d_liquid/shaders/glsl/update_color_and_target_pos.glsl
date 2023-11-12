#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};
layout (set = 2, binding = 0) uniform texture2D t_bg;
layout (set = 2, binding = 1) uniform sampler   s_bg;

layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
void main() {
    Particle particle = particle_data[gl_GlobalInvocationID.x];
    particle.target_pos = particle.pos;
    vec2 bg_uv = particle.pos/5.0 * 1./vec2(constants.aspect, -1.0) * 0.5 + 0.5;
    particle.clr = texture(sampler2D(t_bg, s_bg), bg_uv).rgb;
    particle_data[gl_GlobalInvocationID.x] = particle;
}
