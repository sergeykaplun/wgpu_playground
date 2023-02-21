#version 450

layout(set = 0, binding = 0) uniform Constants {
    vec2 shadow_res;
    vec2 light_position;
    vec3 light_color;
    float time;
    vec2 cells_cnt;
    vec2 unused;
} constants;
layout(set = 1, binding = 0) uniform Camera {
    mat4 view_proj;
} camera;

layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;
layout (location = 2) in vec3 normal;
layout (location = 3) in vec4 offset;

layout (location = 0) out vec3 world_pos;
layout (location = 1) out float diffuse_amount;

void main() {
    world_pos = position * vec3(0.5, 1.0, 0.5) + vec3(offset.x, 0.0, offset.y) + vec3(offset.z, 0.0, offset.w);
    
    gl_Position = camera.view_proj * vec4(world_pos, 1.0);
    vec3 light_xz = vec3(constants.light_position.x, 0.0, constants.light_position.y);
    diffuse_amount = max(dot(normal, normalize(light_xz - world_pos)), 0.)
                       * pow(1.0 - distance(light_xz, world_pos) / 40., 1.5);
}