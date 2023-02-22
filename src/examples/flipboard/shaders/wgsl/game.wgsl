@group(0) @binding(0) var t_output: texture_storage_2d<r32float, write>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord: vec2<i32> = vec2<i32>(workgroupID.xy) * vec2(16) + vec2<i32>(localInvocationID.xy);
    
    let uv = vec2<f32>(tex_coord.xy) / vec2(256.);
    var clr = smoothstep(.3, .25, distance(uv, vec2(0.5)));
    textureStore(t_output, tex_coord, vec4<f32>(clr));
}