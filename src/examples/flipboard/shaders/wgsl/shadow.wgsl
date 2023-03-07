const MIN_FLAP_VAL: f32 = 1e-7;

struct LightData {
    view_proj:          mat4x4<f32>,
    position:           vec4<f32>
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) flap_scale: vec2<f32>,
    @location(2) flap_pos: vec2<f32>,
    @builtin(instance_index) instance_id: u32,
};
struct Cell {
  cur_val: f32,
  next_val: f32,
};

@group(0) @binding(2) var<uniform> light: LightData;
@group(1) @binding(0) var<storage, read> game_input: array<Cell>;

@vertex
fn shadow(in: VertexInput) -> @builtin(position) vec4<f32> {
    let cell_index = in.instance_id/3u;
    let cell = game_input[cell_index];
    let is_flapping = fract(cell.cur_val) != 0.0;
    
    var pos = vec3(in.position.xy * in.flap_scale * .95, 0.0);
    switch (in.instance_id % 3u) {
        case 0u: {
            pos = pos + vec3(in.flap_pos, 0.0);
            break;
        }
        case 1u: {
            pos = -pos + vec3(in.flap_pos, 0.0);
            break;
        }
        case 2u: {
            let rotmat = rotate_x(abs(fract(cell.cur_val)) * 3.141592);
            pos = pos * rotmat + vec3(in.flap_pos, 0.0001);
            pos *= select(0.0, 1.0, is_flapping);               // degenerate
            break;
        }
        default: {
            break;
        }
    }
    
    return light.view_proj * vec4(pos, 1.0);
}

fn rotate_x(angle: f32) -> mat3x3<f32>{
    return rotate_along(vec3(1.0, 0.0, 0.0), angle);
}

fn rotate_along(axis: vec3<f32>, angle: f32) -> mat3x3<f32>{
    let s = sin(angle);
    let c = cos(angle);
    let oc = 1.0 - c;
    
    return mat3x3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                  oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                  oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c);
}