#version 450

layout(set = 0, binding = 0) uniform Constants {
    vec2 shadow_res;
    vec2 light_position;
    vec3 light_color;
    float time;
    vec2 cells_cnt;
    vec2 unused;
} constants;
layout (set = 2, binding = 0) uniform texture2D t_shadow;
layout (set = 2, binding = 1) uniform sampler   s_shadow;

layout (location = 0) in vec2 uv;
layout (location = 0) out vec4 res;

void main() {
    float shadow = texture(sampler2D(t_shadow, s_shadow), uv).r;
    vec2 light_xz = constants.light_position;
    vec2 world_pos = in.uv * 40. - 20.;
    float lighted = pow(1.0 - distance(light_xz, world_pos) / 40., 1.5);
    
    res = vec4(constants.light_color, 1.0) * lighted * shadow;

    float light_pos_mark = smoothstep(.0015, .005, distance(distance(in.uv, light_xz), .01));
    res = mix(vec4(1., 0., 0., 1.), res, light_pos_mark);

    // vec2 mod_uv = fract(in.uv * constants.cells_cnt);
    // float grid = smoothstep(vec2(.01), vec2(.03), mod_uv);
    // res = mix(vec4(1.), res, min(grid.x, grid.y));
}