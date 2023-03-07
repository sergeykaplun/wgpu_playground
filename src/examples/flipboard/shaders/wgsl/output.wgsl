const MIN_FLAP_VAL: f32 = 1e-7;
struct Constants {
    game_res:           vec2<f32>,
    time:               f32,
    time_delta:         f32,
};
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
    //@location(3) albedo: f32,
    @location(4) shadow_pos: vec3<f32>,
    @location(5) id: u32,
    @location(6) next_id: u32,
};

@group(0) @binding(0) var<uniform> constants: Constants;
@group(0) @binding(1) var<uniform> camera: CameraUniform;
@group(0) @binding(2) var<uniform> light: LightData;

struct Cell {
  cur_val: f32,
  next_val: f32,
};
@group(1) @binding(0) var<storage, read> game_input: array<Cell>;
//TODO texture cann be single channel
@group(1) @binding(1) var t_font_atlas: texture_2d<f32>;
@group(1) @binding(2) var s_font_atlas: sampler;

@group(2) @binding(0) var t_shadow: texture_depth_2d;
@group(2) @binding(1) var sampler_shadow: sampler_comparison;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.nrm = in.normal;
    
    let cell_index = in.instance_id/3u;
    let cell = game_input[cell_index];
    let is_flapping = fract(cell.cur_val) != 0.0;

    var pos = vec3(in.position.xy * in.flap_scale * .95, 0.0);
    switch (in.instance_id % 3u) {
        case 0u: {
            pos = pos + vec3(in.flap_pos, 0.0);
            out.id = u32(cell.next_val);
            out.uv = vec2(in.position.x * 0.5 + 0.5, (1.0 - in.position.y) * 0.5);
            break;
        }
        case 1u: {
            pos = -pos + vec3(in.flap_pos, 0.0);
            out.id = u32(floor(cell.cur_val));
            out.uv = vec2(1.0 - (in.position.x * 0.5 + 0.5), 0.5 + 0.5 - (1.0 - in.position.y) * 0.5);
            break;
        }
        case 2u: {
            if is_flapping {
                let rotmat = rotate_x(abs(fract(cell.cur_val)) * 3.141592);
                pos = pos * rotmat + vec3(in.flap_pos, 0.0001);
                out.nrm = rotmat * in.normal;
            } else {
                // degenerate
                pos = vec3(0.0);
            }
            out.uv = vec2(in.position.x * 0.5 + 0.5, (1.0 - in.position.y) * 0.5);
            out.id = u32(floor(cell.cur_val));
            out.next_id = u32(floor(cell.next_val));
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
    let TEAL = vec4(0.0, 128.0, 128.0, 255.0)/255.;
    let CORAL = vec4(255.0, 127.0, 80.0, 255.0)/255.;
    
    let light_dir = normalize(light.position.xyz - in.world_pos);
    var diffuse = max(dot(select(in.nrm, -in.nrm, !is_ff), light_dir), 0.0);
    var shadow = textureSampleCompare(t_shadow, sampler_shadow, in.shadow_pos.xy, in.shadow_pos.z);

    var uv = in.uv;
    if (!is_ff) {
        uv.y = 0.5 + 0.5 - uv.y;
    }
    return mix(TEAL, CORAL, get_char(f32(select(in.next_id, in.id, is_ff)), uv)) * max(diffuse * shadow, 0.25);
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

fn get_char(id: f32, uv: vec2<f32>) -> f32{
    let CHAR_ATLAS_SIZE = 8.0;
    let LETTER_SCALE = vec2(0.125);

    let POS_IN_ATLAS = vec2(floor(id % CHAR_ATLAS_SIZE), floor(id / CHAR_ATLAS_SIZE));
    let final_uv = LETTER_SCALE * POS_IN_ATLAS + uv * LETTER_SCALE;
    return smoothstep(0.4, 0.45, textureSample(t_font_atlas, s_font_atlas, final_uv).r);
}