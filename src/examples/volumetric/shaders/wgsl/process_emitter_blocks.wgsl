struct PushConstants {
    iTime: f32,
    resolution: i32,
    emitter_data: vec4<f32>
}
struct IndirectDispatchArguments {
    x: atomic<u32>,
    y: u32,
    z: u32,
};

@group(0) @binding(0) var<uniform> constants : PushConstants;
@group(1) @binding(0) var<storage, write> indirectDispatchBuffer: IndirectDispatchArguments;
var<workgroup> toEmitCounter: atomic<u32>;

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
       @builtin(local_invocation_index) localInvocationIndex: u32/*, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(4) + vec3<i32>(localInvocationID);
    let uv = vec3<f32>(tex_coord) / vec3(f32(constants.resolution));
    /*var v1: vec3<i32> = vec3<i32>(tex_coord);
    var v2: vec3<i32> = vec3<i32>(constants.resolution/2);
    var diff: vec3<i32> = v1 - v2;
    if (all(abs(diff) < 4))*/
    if (distance(uv, constants.emitter_data.rgb) < constants.emitter_data.a)
    {
        atomicAdd(&toEmitCounter, 1u);
    }
    workgroupBarrier();
    if (localInvocationIndex == 0u && toEmitCounter > 0u)
    {
        let curIndex = atomicAdd(&indirectDispatchBuffer.x, 1u);
    }
}