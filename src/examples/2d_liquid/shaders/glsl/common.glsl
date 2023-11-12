const float PI = 3.1415926535897932384626433832795;
const float TAU = PI * 2.0;
const uint MAX_UINT = 0xFFFFFFFFu;

struct Particle {
    vec2 pos;
    vec2 predicted_pos;
    vec2 vel;
    vec2 target_pos;
    vec3 clr;
    float density;
};

struct Constants {
    vec2 gravity;
    float smoothing_radius;
    float particle_mass;

    float aspect;
    uint particle_segments;
    float particle_radius;
    float delta_time;

    vec2 bounds_size;
    float damping;
    uint particles_count;

    float target_density;
    float pressure_multiplier;
    vec2 pointer_location;

    vec2 resolution;
    float pointer_active;
    float pointer_attract;
};

struct SortingParams {
    uint group_width;
    uint group_height;
    uint step_index;
};

struct SpatialLookupItem {
    uint particle_id;
    uint cell_key;
};

float smooth_kernel(float dst, float radius) {
    if(dst >= radius) { return 0.0; }

    float volume = PI * pow(radius, 4.0) / 6.0;
    return (radius - dst) * (radius - dst) / volume;
}

float smooth_kernel_derivative(float dst, float radius) {
    if (dst >= radius) { return 0.0; }
    float scale = 12. / (PI * pow(radius, 4.0));
    return scale * (dst - radius);
}

#define DENS_2_PRESS(dens, target_dens, mult) ((dens - target_dens) * mult)
#define PARTICLE_CELL(pos) (ivec2(ceil(pos/constants.smoothing_radius)))
#define CELL_HASH(cell) uint(cell.x * 19349 + cell.y * 73856)
#define CELL_KEY(hash) (hash % constants.particles_count)