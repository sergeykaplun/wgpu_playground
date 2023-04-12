struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    out.clip_pos = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

@group(0) @binding(0) var t_checker: texture_2d<f32>;
@group(0) @binding(1) var s_checker: sampler;
@group(1) @binding(0) var<uniform> cur_mip: vec4<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let flipped_uv = vec2(in.uv.x, 1.0 - in.uv.y);
    return textureSampleLevel(t_checker, s_checker, flipped_uv, cur_mip.x);
}