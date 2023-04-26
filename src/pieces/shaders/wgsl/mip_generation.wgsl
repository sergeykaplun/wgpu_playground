struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec4<f32>
};

@group(0) @binding(0) var t_skybox: texture_cube<f32>;
@group(0) @binding(1) var s_skybox: sampler;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(position, 1.0);
    out.world_pos = model_matrix * out.clip_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //var to_fragment = (model_matrix * normalize(in.world_pos)).xyz;
    var to_fragment = normalize(in.world_pos.xyz);
    return textureSample(t_skybox, s_skybox, to_fragment);
}