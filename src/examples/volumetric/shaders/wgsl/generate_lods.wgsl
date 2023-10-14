@group(0) @binding(0) var read_target: texture_3d<f32>;
@group(0) @binding(1) var write_target: texture_storage_3d<rgba16float, write>;

var<workgroup> toEmitCounter: atomic<u32>;

@compute @workgroup_size(2, 2, 2)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32/*, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let source_tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(2) + vec3<i32>(localInvocationID);
    let dest_tex_coord: vec3<i32> = vec3<i32>(workgroupID);

    let val = textureLoad(read_target, tex_coord, 0);
    if (any(val > 0.0))
    {
        atomicAdd(&toEmitCounter, 1u);
    }
    workgroupBarrier();
    if (localInvocationIndex == 0u && toEmitCounter > 0u)
    {
        textureStore(write_target, dest_tex_coord, 1);
    }
}