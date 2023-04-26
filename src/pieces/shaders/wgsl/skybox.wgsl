struct CameraParams {
    projection :                      mat4x4<f32>,
    model :                           mat4x4<f32>,
    view :                            mat4x4<f32>,
    position :                        vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>
};

@group(0) @binding(0) var t_skybox: texture_cube<f32>;
@group(0) @binding(1) var s_skybox: sampler;
@group(1) @binding(0) var<uniform> camera: CameraParams;

@vertex
fn vs_main(@location(0) position: vec3<f32>,) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = position * 10.0;
    out.clip_pos = camera.projection * camera.view * vec4<f32>(out.world_pos, 1.0);
    out.clip_pos = out.clip_pos.xyww;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var to_fragment = normalize(in.world_pos);
    return textureSample(t_skybox, s_skybox, to_fragment);
}