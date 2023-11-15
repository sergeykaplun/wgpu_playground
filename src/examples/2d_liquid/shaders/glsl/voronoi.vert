#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};

layout(location = 0) out vec2 uv;
layout(location = 1) out vec2 simulation_space_uv;

void main() {
    uv = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    simulation_space_uv = (uv * 2.0 - 1.0) * vec2(constants.aspect, 1.0) * 5.;
}
