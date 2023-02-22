struct Globals {
    unused_here:        vec2<f32>,
    game_res:           vec2<f32>,
    time:               f32,
    //unused:             vec3<f32>
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<storage, read_write> game_output: array<vec2<f32>>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>) {
    let tex_coord: vec2<i32> = vec2<i32>(workgroupID.xy) * vec2(16) + vec2<i32>(localInvocationID.xy);
    let id = tex_coord.y * i32(globals.game_res.x) + tex_coord.x;
    
    let uv = vec2<f32>(tex_coord.xy) / globals.game_res;
    //var clr = step(distance(uv, vec2(sin(globals.time) * .1, 0.5)), .25);
    var clr = step(uv.x, 0.5 + sin(globals.time) * 0.5);
    //var clr = 1.0;
    //if((id + i32(globals.time)) % 2 == 0) {
    //    clr = 0.0;
    //}

    //textureStore(t_output, tex_coord, vec4<f32>(clr));
    game_output[id] = vec2(clr);
}