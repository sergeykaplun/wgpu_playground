const MIN_FLAP_VAL: f32 = 1e-7;

struct Globals {
    unused_here:        vec2<f32>,
    game_res:           vec2<f32>,
    time:               f32,
    time_delta:         f32,
    unused:             vec2<f32>
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<storage, read_write> game_output: array<vec2<f32>>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>) {
    let tex_coord: vec2<i32> = vec2<i32>(workgroupID.xy) * vec2(16) + vec2<i32>(localInvocationID.xy);
    let id = tex_coord.y * i32(globals.game_res.x) + tex_coord.x;
    
    let prev_val = game_output[id].r;
    let a = abs(prev_val);
    let s = sign(prev_val);
    if(a > MIN_FLAP_VAL) {                                                              // if during flap
        if(a == 1.0){                                                                   // down
            game_output[id] = vec2(-s * MIN_FLAP_VAL);                                  // invert
        } else {
            let new_val = min(abs(prev_val + s * globals.time_delta), 1.0) * s;         // increase val
            game_output[id] = vec2(new_val);
        }
    } else {
        let uv = vec2<f32>(tex_coord.xy) / globals.game_res;
        var clr = sign(get_pattern(uv) - 0.5);
        if(s != clr) {
            let new_val = prev_val + s * globals.time_delta;                            // start flap
            game_output[id] = vec2(new_val);
        }
    }
}

fn get_pattern(in_uv: vec2<f32>) -> f32 {
    // let a = globals.time * .25;
    // let r_uv = mat2x2(cos(a), -sin(a), sin(a), cos(a)) * (in_uv - 0.5) + vec2(0.5);
    // let uv = abs(r_uv - 0.5);
    
    // let diagon = step(uv.y, uv.x + .05) * step(uv.x - .05, uv.y);
    // return diagon;

    return step(distance(in_uv, vec2(0.5 + sin(globals.time * 0.25) * 0.25, 0.5)), .25);
}