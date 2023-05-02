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

//const PI: f32 = 3.1415926535897932384626433832795;
const PI: f32 = 3.141592;
const TWO_PI: f32 = 6.28318530718;
const HALF_PI: f32 = 1.5707964;

//TODO to be configurable
const DELTA_PHI: f32 = 0.03490658503;
const DELTA_THETA: f32 = 0.02454369375;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let to_fragment = normalize(in.world_pos.xyz);
    var up = vec3(0.0, 1.0, 0.0);
    let right = cross(up, to_fragment);
    up = cross(to_fragment, right);

    var color : vec3<f32> = vec3<f32>(0.0);
    var sampleCount : u32 = 0u;
    for (var phi : f32 = 0.0; phi < TWO_PI; phi += DELTA_PHI) {
        for (var theta : f32 = 0.0; theta < HALF_PI; theta += DELTA_THETA) {
            let tempVec : vec3<f32> = cos(phi) * right + sin(phi) * up;
            let sampleVector : vec3<f32> = cos(theta) * to_fragment + sin(theta) * tempVec;
            color += textureSample(t_skybox, s_skybox, sampleVector).rgb * cos(theta) * sin(theta);
            sampleCount++;
        }
    }
    return vec4<f32>(PI * color / f32(sampleCount), 1.0);
}