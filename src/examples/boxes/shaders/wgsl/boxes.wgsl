struct GlobalConstants
{
    shadow_res:         vec2<f32>,
    light_position:     vec2<f32>,
    light_color:        vec3<f32>,
    time:               f32,
    cells_cnt:          vec2<f32>,
    unused:             vec2<f32>
};
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) offset: vec4<f32>
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) diffuse_amount: f32,
};

@group(0) @binding(0) var<uniform> constants: GlobalConstants;
@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var t_shadow: texture_2d<f32>;
@group(2) @binding(1) var s_shadow: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = in.position * vec3(0.5, 1.0, 0.5) + vec3(in.offset.x, 0.0, in.offset.y) + vec3(in.offset.z, 0.0, in.offset.w);
    out.clip_pos = camera.view_proj * vec4<f32>(out.world_pos, 1.0);
    let light_xz = vec3(constants.light_position.x, 0.0, constants.light_position.y);
    out.diffuse_amount = max(dot(in.normal, normalize(light_xz - out.world_pos)), 0.)
                       * pow(1.0 - distance(light_xz, out.world_pos) / 40., 1.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coords = (in.world_pos.xz / 40.) + 0.5;
    let shadow = textureSample(t_shadow, s_shadow, tex_coords).x;
    return vec4(constants.light_color * in.diffuse_amount, 1.0) * shadow;
}