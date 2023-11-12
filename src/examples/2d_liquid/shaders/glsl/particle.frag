#version 450

layout (location = 0) in vec3 clr;
layout (location = 0) out vec4 res;

void main() {
    res = vec4(clr, 1.0);
}