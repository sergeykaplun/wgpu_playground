@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;
@group(1) @binding(0) var t_output: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(0) var<uniform> resolution: vec4<f32>;

//@compute @workgroup_size(16, 16, 1)
@compute @workgroup_size(8, 8, 1)
fn generate_mip(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord: vec2<i32> = vec2<i32>(workgroupID.xy) * vec2(8) + vec2<i32>(localInvocationID.xy);
    let res = textureDimensions(t_output);
    //if(all(vec2<f32>(tex_coord) < resolution.xy)) {
    if(all(tex_coord < res.xy)) {
        let uv: vec2<f32> = vec2<f32>(tex_coord.xy) / vec2<f32>(res.xy);
        
        let tl_uv = floor(uv);
        let tr_uv = tl_uv + vec2<f32>(1.0, 0.0);
        let bl_uv = tl_uv + vec2<f32>(0.0, 1.0);
        let br_uv = tl_uv + vec2<f32>(1.0, 1.0);
        
        // Calculate the texel values for bilinear filtering
        let tl_texel = textureLoad(t_source, vec2<i32>(i32(tl_uv.x), i32(tl_uv.y)), 0);
        let tr_texel = textureLoad(t_source, vec2<i32>(i32(tr_uv.x), i32(tr_uv.y)), 0);
        let bl_texel = textureLoad(t_source, vec2<i32>(i32(bl_uv.x), i32(bl_uv.y)), 0);
        let br_texel = textureLoad(t_source, vec2<i32>(i32(br_uv.x), i32(br_uv.y)), 0);
        
        // Calculate the interpolated texel value
        let f = fract(uv);
        let top = mix(tl_texel, tr_texel, f.x);
        let bot = mix(bl_texel, br_texel, f.x);
        let texel = mix(top, bot, f.y);
        textureStore(t_output, tex_coord, texel);

        //let env = textureLoad(t_source, s_source, uv);
        //let env = vec4(1.0, 1.0, 0.1, 1.0);
        //textureStore(t_output, tex_coord, env);
    }
    //if (all(tex_coord < resolution.xy)) 
    // {
    //     let uv = vec2<f32>(tex_coord.xy) / resolution.xy;
    //     //let env = vec4(1.0, 1.0, 0.1, 1.0);
    //     //let env = textureLoad(t_source, s_source, uv);
    //     let env = textureSample(t_source, s_source, vec2(0.5));
    //     textureStore(t_output, tex_coord, env);
    // }
}