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

layout (location = 0) in vec3 world_pos;
layout (location = 1) in float diffuse_amount;
layout (location = 0) out vec4 res;

void main() {
    vec2 tex_coords = (world_pos.xz / 40.) + 0.5;
    float shadow = texture(sampler2D(t_shadow, s_shadow), tex_coords).r;
    res = vec4(constants.light_color * diffuse_amount, 1.0) * shadow;
}