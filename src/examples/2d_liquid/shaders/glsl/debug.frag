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

layout(location = 0) in vec2 uv;
layout(location = 1) in vec2 simulation_space_uv;
layout(location = 0) out vec4 res;

float bound(vec2 uv) {
    float width = constants.particle_radius * 0.25;
    vec2 half_bounds = constants.bounds_size * 0.5 + width;
    float mask = max(step(distance(abs(uv.x), half_bounds.x), width) * step(abs(uv.y), half_bounds.y + width),
                     step(distance(abs(uv.y), half_bounds.y), width) * step(abs(uv.x), half_bounds.x + width));
    return mask;
}

float grid() {
    return step(0.485, max(abs(mod(simulation_space_uv.x, constants.smoothing_radius)/constants.smoothing_radius - 0.5),
                           abs(mod(simulation_space_uv.y, constants.smoothing_radius)/constants.smoothing_radius - 0.5)));
}

struct VoronoiRes {
    float dst;
    uint id;
};

VoronoiRes vor(vec2 pos) {
    float res = 1000.;
    uint res_id = 0;
    ivec2 center = PARTICLE_CELL(pos);
    for(int i = -1; i <= 1; i++) {
        for(int j = -1; j <= 1; j++) {
            ivec2 cell_coord = center + ivec2(i, j);
            //TODO enable
            //if(any(cell_coord < vec2(0)) && any(cell_coord >= vec2<i32>(ceil(constants.bounds_size / constants.smoothing_radius)))) {
            //    continue;
            //}
            uint hash = CELL_HASH(cell_coord);
            uint key = CELL_KEY(hash);

            uint cellStartIndex = start_indices[key];
            for (uint i=cellStartIndex; i<constants.particles_count; i++) {
                if(spatial_lookup[i].cell_key != key)  break;
                uint particle_index = spatial_lookup[i].particle_id;
                Particle particle = particle_data[particle_index];

                float dst = distance(particle.pos, pos);
                if (res > dst) {
                    res = dst;
                    res_id = particle_index;
                }
            }
        }
    }
    return VoronoiRes(res, res_id);

    /*float res = 1000.;
    uint res_id = 0;
    for (uint i = 0; i < constants.particles_count; i++) {
        float dst = distance(pos, particle_data[i].pos);
        if (res > dst) {
            res = dst;
            res_id = i;
        }
    }
    return VoronoiRes(res, res_id);*/
}

void main() {
    /*vec3 clr = vec3(.1) * grid();
    clr = mix(clr, vec3(0.8, 0.125, 0.2), bound(simulation_space_uv));
    if (constants.pointer_active > 0.0)
    {
        vec2 pl = (constants.pointer_location/constants.resolution * 2.0 - 1.0) * 5.0 * vec2(constants.aspect, -1.0);
        clr = mix(clr, vec3(0.0, 1.0, 0.0), smoothstep(0.005, 0.0025, distance(distance(simulation_space_uv, pl), constants.smoothing_radius)));
    }
    res = vec4(clr, 1.0);
    */

    VoronoiRes vor_res = vor(simulation_space_uv);
    //uint particle_id = vor(simulation_space_uv);
    Particle particle = particle_data[vor_res.id];

    float speed = length(particle.vel);
    vec3 clr = particle.clr * (1. - distance(simulation_space_uv, particle.pos)/.15);
    clr = pow(clr, vec3(1. + 1.5 * smoothstep(0., 2.5, speed)));
    res = vec4(clr, 1.0);
}
