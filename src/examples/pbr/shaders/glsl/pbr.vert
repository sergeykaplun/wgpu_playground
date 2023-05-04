#version 450

layout(set = 0, binding = 0) uniform CameraParams {
    mat4 projection;
    mat4 model;
    mat4 view;
    vec4 position;
} camera_params;
layout(set = 2, binding = 0) uniform UBONode {
    mat4 transform;
    //mat4 jointMatrix[64];
    //float jointCount;
} node;

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec2 in_uv0;
layout (location = 3) in vec2 in_uv1;
layout (location = 4) in vec3 in_color;

layout (location = 0) out vec3 out_world_pos;
layout (location = 1) out vec3 out_normal;
layout (location = 2) out vec2 out_uv0;
layout (location = 3) out vec2 out_uv1;
layout (location = 4) out vec3 out_color;

void main() {
    out_color = in_color;

    vec4 loc_pos = camera_params.model * node.transform * vec4(in_pos, 1.0);
    out_normal = normalize(transpose(inverse(mat3(camera_params.model * node.transform))) * in_normal);

    //locPos.y = -locPos.y;
    out_world_pos = loc_pos.xyz / loc_pos.w;
    out_uv0 = in_uv0;
    out_uv1 = in_uv1;
    gl_Position = camera_params.projection * camera_params.view * vec4(out_world_pos, 1.0);
}