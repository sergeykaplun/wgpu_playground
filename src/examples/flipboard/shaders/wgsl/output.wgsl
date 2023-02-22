struct Globals {
    output_res:         vec2<f32>,
    input_res:          vec2<f32>,
    time:               f32,
    //unused:             vec3<f32>
};
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) flap_scale: vec2<f32>,
    @location(3) flap_pos: vec2<f32>,
    @builtin(instance_index) instance_id: u32
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) id: u32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<storage, read> game_input: array<vec2<f32>>;
@group(2) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    var pos = vec3(in.position.xy * in.flap_scale, 0.0);
    if(in.instance_id % 3u == 2u) {
        pos = vec3(0.0);//pos * rotationMatrix(vec3(1.0, 0.0, 0.0), globals.time % 3.1415) + vec3(in.flap_pos, 0.0);
    } else {
        pos = pos + vec3(in.flap_pos, 0.0);
    }
    out.clip_pos = camera.view_proj * vec4(pos, 1.0);
    out.id = in.instance_id/3u;
    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_ff: bool ) -> @location(0) vec4<f32> {
    return vec4(f32(in.id)/(32.0*32.0));
    // if is_ff {
    //     return vec4(game_input[in.id].r);
    // } else {
    //     return vec4(0.3, 0.5, 0.7, 1.0);
    // }
}

fn rotationMatrix(axis: vec3<f32>, angle: f32) -> mat3x3<f32>{
    let axis_n = normalize(axis);
    let s = sin(angle);
    let c = cos(angle);
    let oc = 1.0 - c;
    
    return mat3x3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                  oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                  oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c);
}