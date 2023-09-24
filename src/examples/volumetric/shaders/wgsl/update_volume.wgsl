@group(0) @binding(0) var volume: texture_storage_3d<rgba16float, write>;
@group(1) @binding(0) var r_volume: texture_3d<f32>;
@group(1) @binding(1) var s_volume: sampler;

@compute @workgroup_size(4, 4, 4)
fn update_volume(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(4) + vec3<i32>(localInvocationID);
    var val = textureLoad(r_volume, tex_coord + vec3(0, -1, 0), 0);
    val.a *= 0.975;
    //let val = textureSample(r_volume, s_volume, vec3<f32>(tex_coord + vec3(0, -1, 0)));
    textureStore(volume, tex_coord, val);
}