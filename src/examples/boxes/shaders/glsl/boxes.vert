#version 450

layout (location = 0) in vec3 position;
layout (location = 0) out vec2 uv;

layout(binding = 0, std140) uniform Camera
{
    mat4 ViewProj;
};

void main() {
    uv = (position.xy + 1.) - .5;
    gl_Position = ViewProj * vec4(position, 1.0);
}