#version 450
#include "common.glsl"

layout(set = 0, binding = 0) uniform ConstantsData {
    Constants constants;
};
layout(set = 1, binding = 0) buffer ParticlesData {
    Particle particle_data[];
};

layout(location = 0) out vec3 clr;

vec3 palette(float h) {
    vec3 col =    vec3(0.0,0.3,1.0);
    col = mix(col, vec3(1.0,0.8,0.0), smoothstep(0.13, 0.53, h));
    col = mix(col, vec3(1.0,0.0,0.0), smoothstep(0.46, 0.86, h));
    col.y += 0.5*(1.0-smoothstep(0.0, 0.2, abs(h - 0.33)));
    col *= 0.5 + 0.5*h;
    return col;
}

void main() {
    const uint SEGMENT_VERTICES = constants.particle_segments * 3;
    const uint particleID = gl_VertexIndex / SEGMENT_VERTICES;
    const uint segmentID = (gl_VertexIndex % SEGMENT_VERTICES) / 3;
    const uint vertexID = (gl_VertexIndex % SEGMENT_VERTICES) % 3;
    const float segmentSpan = TAU / constants.particle_segments;

    vec2 offset = vec2(0.0);
    switch (vertexID) {
        case 1:
            offset = vec2(constants.particle_radius * cos((segmentID + 1.0) * segmentSpan),
                          constants.particle_radius * sin((segmentID + 1.0) * segmentSpan));
            break;
        case 2:
            offset = vec2(constants.particle_radius * cos(segmentID * segmentSpan),
                          constants.particle_radius * sin(segmentID * segmentSpan));
            break;
        default:
            offset = vec2(0.0, 0.0);
            break;
    }

    const Particle particle = particle_data[particleID];
    gl_Position = vec4((particle.pos + offset)/vec2(constants.aspect, 1.0) * 0.2, 0.0, 1.0);

    /*ivec2 particle_cell = PARTICLE_CELL(particle.pos);
    uint particle_hash = CELL_HASH(particle_cell);
    uint particle_key = CELL_KEY(particle_hash);

    vec2 pl = (constants.pointer_location/constants.resolution * 2.0 - 1.0) * 5.0 * vec2(constants.aspect, -1.0);
    ivec2 pointer_cell = PARTICLE_CELL(pl);
    uint pointer_hash = CELL_HASH(pointer_cell);
    uint pointer_key = CELL_KEY(pointer_hash);

    clr = (particle_hash == pointer_hash) ? vec3(1.0, 0.0, 0.0) : vec3(1.0, 1.0, 0.0);*/

    float speed = length(particle.vel);
    clr = palette(smoothstep(0., 2.5, speed));

    //clr = particle.clr;
}