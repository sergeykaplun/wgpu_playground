struct PushConstants {
    iTime: f32,
}
@group(0) @binding(0) var volume: texture_storage_3d<rgba16float, read_write>;
/*@group(0) @binding(1) var r_volume: texture_3d<f32>;
@group(0) @binding(2) var s_volume: sampler;*/
@group(1) @binding(0) var<uniform> constants : PushConstants;

@compute @workgroup_size(4, 4, 4)
fn update_volume(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(4) + vec3<i32>(localInvocationID);
    let val = textureLoad(volume, tex_coord + vec3(0, -1, 0));
    //let texSize = vec3(512);
    //let uvw = vec3<f32>(tex_coord + vec3(0, -1, 0)) / vec3<f32>(texSize);
    //let val = tricubicInterpolation(/*volume, */uvw, texSize);
    //let val = tricubicInterpolation(tex_coord + vec3(0, -1, 0));
    textureStore(volume, tex_coord, val);
}

// Define a function to perform tricubic interpolation without a sampler.
fn tricubicInterpolation(/*texture: texture_3d<f32>,*/ uvw: vec3<f32>, texSize: vec3<i32>) -> vec4<f32> {
    var p0: vec3<i32> = clamp(vec3<i32>(floor(uvw * vec3<f32>(texSize))), vec3<i32>(0), texSize - vec3<i32>(1));
    var frac: vec3<f32> = fract(uvw * vec3<f32>(texSize));
    //var frac: vec3<f32> = fract(uvw * texSize);

    var result: vec4<f32> = vec4<f32>(0.0);
    for (var dz: i32 = -1; dz <= 2; dz++) {
        for (var dy: i32 = -1; dy <= 2; dy++) {
            for (var dx: i32 = -1; dx <= 2; dx++) {
                var offset: vec3<i32> = p0 + vec3<i32>(dx, dy, dz);
                var texel = textureLoad(volume, offset);
                var weight: f32 = cubicWeight(frac.x - f32(dx), frac.y - f32(dy), frac.z - f32(dz));
                result += weight * texel;
            }
        }
    }

    return result;
}

// Cubic interpolation weights
fn cubicWeight(t: f32, t1: f32, t2: f32) -> f32 {
    let a = -0.5;
    let t3 = t * t * t;
    let t1_3 = t1 * t1 * t1;
    let t2_3 = t2 * t2 * t2;

    return a * (t3 - 2.0 * t1_3 + t2_3);
}