#version 450

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 res;

void main()
{
    res = vec4(uv, 0., 1.);
}