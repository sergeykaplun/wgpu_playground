struct PushConstants {
    iTime: f32,
    resolution: u32,
    emitter_data: vec4<f32>
}
struct EmitterArgs {
    dispatch_offset: vec3<u32>,
}
@group(0) @binding(0) var t_output: texture_storage_3d<rgba16float, write>;
@group(1) @binding(0) var<uniform> constants : PushConstants;
@group(2) @binding(0) var<uniform> emitter_args : EmitterArgs;

fn saturate(val: vec3<f32>) -> vec3<f32> {
    return clamp(val, vec3(0.), vec3(1.));
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let rgb = saturate(abs((c.x * 6.0 + vec3(0.0, 4., 2.) % 6.) - 3.) - 1.);
    return c.z * mix(vec3(1.), rgb, c.y);
}

@compute @workgroup_size(4, 4, 4)
fn emit(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(4) + vec3<i32>(localInvocationID) + vec3<i32>(emitter_args.dispatch_offset);
    let uv = vec3<f32>(tex_coord) / vec3(f32(constants.resolution));
    let mask = step(distance(uv, constants.emitter_data.rgb), constants.emitter_data.a);
    if(mask > 0.0)
    {
        var clr = vec4(hsv2rgb(vec3(fract(constants.iTime * 0.2), 1., 1.)), mask);
        textureStore(t_output, tex_coord, clr);
    }
}