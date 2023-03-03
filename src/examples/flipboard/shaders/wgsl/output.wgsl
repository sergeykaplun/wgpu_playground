const MIN_FLAP_VAL: f32 = 1e-7;
// struct Constants {
//     game_res:           vec2<f32>,
//     time:               f32,
//     time_delta:         f32,
// };
struct CameraUniform {
    view_proj:          mat4x4<f32>,
};
struct LightData {
    view_proj:          mat4x4<f32>,
    position:           vec4<f32>
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) flap_scale: vec2<f32>,
    @location(4) flap_pos: vec2<f32>,
    @builtin(instance_index) instance_id: u32,
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) nrm: vec3<f32>,
    @location(3) albedo: f32,
    @location(4) shadow_pos: vec3<f32>,
};

//@group(0) @binding(0) var<uniform> constants: Constants;
@group(0) @binding(1) var<uniform> camera: CameraUniform;
@group(0) @binding(2) var<uniform> light: LightData;

@group(1) @binding(0) var<storage, read> game_input: array<vec2<f32>>;
@group(2) @binding(0) var t_shadow: texture_depth_2d;
@group(2) @binding(1) var sampler_shadow: sampler_comparison;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    out.nrm = in.normal;
    
    let cell_index = in.instance_id/3u;
    let in_val = game_input[cell_index].r;
    let is_flapping = abs(in_val) > MIN_FLAP_VAL;
    let cur_clr = step(0.0, in_val);
    let next_clr = 1.0 - cur_clr;

    var pos = vec3(in.position.xy * in.flap_scale * .95, 0.0);
    switch (in.instance_id % 3u) {
        case 0u: {
            pos = pos + vec3(in.flap_pos, 0.0);
            // TODO rewrite to lerp
            if is_flapping {
                out.albedo = next_clr;
            } else {
                out.albedo = cur_clr;
            }
            break;
        }
        case 1u: {
            pos = -pos + vec3(in.flap_pos, 0.0);
            out.albedo = cur_clr;
            break;
        }
        case 2u: {
            // TODO rewrite to lerp
            if is_flapping {
                let rotmat = rotate_x(abs(in_val) * 3.141592);
                pos = pos * rotmat + vec3(in.flap_pos, 0.0001);
                out.albedo = cur_clr;
                out.nrm = rotmat * in.normal;
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
    out.world_pos = pos;
    out.clip_pos = camera.view_proj * vec4(pos, 1.0);
    
    let pos_from_light = light.view_proj * vec4(out.world_pos, 1.0);
    out.shadow_pos = vec3(
        pos_from_light.xy * vec2(0.5, -0.5) + vec2(0.5),
        pos_from_light.z
    );

    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_ff: bool ) -> @location(0) vec4<f32> {
    let DARK = vec4(0.2);
    let LIGHT = vec4(0.8);
    let TEAL = vec4(0.0, 128.0, 128.0, 255.0)/255.;
    let CORAL = vec4(255.0, 127.0, 80.0, 255.0)/255.;
    
    let val = select(1.0 - in.albedo, in.albedo, is_ff);
    let albedo = mix(TEAL, CORAL, val);

    let light_dir = normalize(light.position.xyz - in.world_pos);
    var diffuse = max(dot(select(in.nrm, -in.nrm, !is_ff), light_dir), 0.0);

    var shadow = textureSampleCompare(t_shadow, sampler_shadow, in.shadow_pos.xy, in.shadow_pos.z);

    return albedo * max(diffuse * shadow, 0.25);
    //return vec4(shadow);
}

fn rotate_x(angle: f32) -> mat3x3<f32>{
    return rotate_along(vec3(1.0, 0.0, 0.0), angle);
}

fn rotate_y(angle: f32) -> mat3x3<f32>{
    return rotate_along(vec3(0.0, 1.0, 0.0), angle);
}

fn rotate_along(axis: vec3<f32>, angle: f32) -> mat3x3<f32>{
    let s = sin(angle);
    let c = cos(angle);
    let oc = 1.0 - c;
    
    return mat3x3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                  oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                  oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c);
}