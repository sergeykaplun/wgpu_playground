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
layout (location = 1) in vec2 in_uv;
layout (location = 0) out vec2 out_uv;

void main() {
    out_uv = in_uv;
    gl_Position = camera.view_proj * vec4(position, 1.0);
}