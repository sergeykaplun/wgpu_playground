const MIN_FLAP_VAL: f32 = 1e-7;

struct Globals {
    unused_here:        vec2<f32>,
    game_res:           vec2<f32>,
    time:               f32,
    time_delta:         f32,
    unused:             vec2<f32>
};
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) flap_scale: vec2<f32>,
    @location(3) flap_pos: vec2<f32>,
    @builtin(instance_index) instance_id: u32,
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) id: u32,
    @location(2) clr: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<storage, read> game_input: array<vec2<f32>>;
@group(2) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    out.id = in.instance_id/3u;
    let in_val = game_input[out.id].r;
    let is_flapping = abs(in_val) > MIN_FLAP_VAL;
    let cur_clr = step(0.0, in_val);
    let next_clr = 1.0 - cur_clr;

    var pos = vec3(in.position.xy * in.flap_scale, 0.0);
    switch (in.instance_id % 3u) {
        case 0u: {
            pos = pos + vec3(in.flap_pos, 0.0);
            // TODO rewrite to lerp
            if is_flapping {
                out.clr = next_clr;
            } else {
                out.clr = cur_clr;
            }
            break;
        }
        case 1u: {
            pos = pos + vec3(in.flap_pos, 0.0);
            out.clr = cur_clr;
            break;
        }
        case 2u: {
            // TODO rewrite to lerp
            if is_flapping {
                pos = pos * rotateX(abs(in_val) * 3.141592) + vec3(in.flap_pos, 0.0001);
                out.clr = cur_clr;
            } else {
                // degenerate
                pos = vec3(0.0);
            }
            break;
        }
        default: {
            break;
        }
    }
    out.clip_pos = camera.view_proj * vec4(pos, 1.0);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_ff: bool ) -> @location(0) vec4<f32> {
    let DARK = vec4(0.1);
    let LIGHT = vec4(0.8);
    let val = select(1.0 - in.clr, in.clr, is_ff);
    return vec4(mix(DARK, LIGHT, val));
    // if is_ff {
    //     return vec4();
    // } else {
    //     return vec4();
    // }
}

fn rotateX(angle: f32) -> mat3x3<f32>{
    let axis = vec3(1.0, 0.0, 0.0);
    let s = sin(angle);
    let c = cos(angle);
    let oc = 1.0 - c;
    
    return mat3x3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                  oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                  oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c);
}