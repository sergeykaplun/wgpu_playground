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
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> constants: GlobalConstants;
@group(1) @binding(0) var<uniform> camera: CameraUniform;
@group(2) @binding(0) var t_shadow: texture_2d<f32>;
@group(2) @binding(1) var s_shadow: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    out.clip_pos = camera.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shadow = textureSample(t_shadow, s_shadow, in.uv).x;
    let light_xz = constants.light_position;
    let world_pos = in.uv * 40. - 20.;
    let lighted = pow(1.0 - distance(light_xz, world_pos) / 40., 1.5);
    
    //var res = vec4(constants.light_color, 1.0) * lighted * shadow;
    var res = vec4(shadow);

    let light_pos_mark = smoothstep(.0015, .005, distance(distance(in.uv, light_xz), .01));
    res = mix(vec4(1., 0., 0., 1.), res, light_pos_mark);

    let mod_uv = fract(in.uv * constants.cells_cnt);
    let grid = smoothstep(vec2(.01), vec2(.03), mod_uv);
    res = mix(vec4(1.), res, min(grid.x, grid.y));

    
    return res;
}