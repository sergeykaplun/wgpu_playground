struct Constants
{
    resolution: vec2<f32>,
    time:       f32,
    unused:     f32
};
 
@group(0) @binding(0) var<uniform> constants: Constants;
@group(0) @binding(1) var t_output: texture_storage_2d<r32float, write>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord = workgroupID.xy * vec2(16u) + localInvocationID.xy;

    textureStore(t_output, tex_coord, vec4<f32>(1000.));
}