struct GlobalConstants
{
    shadow_res:         vec2<f32>,
    light_position:     vec2<f32>,
    light_color:        vec3<f32>,
    time:               f32,
    cells_cnt:          vec2<f32>,
    unused:             vec2<f32>
};

@group(0) @binding(0) var<uniform> constants: GlobalConstants;
@group(1) @binding(0) var t_output: texture_storage_2d<r32float, write>;
@group(1) @binding(1) var<storage, read> cells_centers : array<vec4<f32>>;

fn rect_hit(ray_origin: vec2<f32>, ray_dir: vec2<f32>, bounds_min: vec2<f32>, bounds_max: vec2<f32>) -> bool {
    let t1 = (bounds_min.x - ray_origin.x) / ray_dir.x;
    let t2 = (bounds_max.x - ray_origin.x) / ray_dir.x;
    let t3 = (bounds_min.y - ray_origin.y) / ray_dir.y;
    let t4 = (bounds_max.y - ray_origin.y) / ray_dir.y;
    
    let tmin = max(min(t1, t2), min(t3, t4));
    let tmax = min(max(t1, t2), max(t3, t4));

    return !(tmin < 0.0 || tmax < 0.0 || tmin > tmax);
}

fn get_inner_rect_for_cell(pos: vec2<f32>) -> vec4<f32> {
    let index = u32(pos.x * constants.cells_cnt.x + pos.y);
    let pos_offset = cells_centers[index].zw * .25;
    let left_up = pos + 0.5 - .125 + pos_offset ;
    let right_bottom = pos + 0.5 + .125 + pos_offset;

    return vec4(left_up, right_bottom);
}

fn calculate_shadows(uv: vec2<f32>) -> f32{
    let LIGHT_POS = constants.light_position * constants.cells_cnt;
    let LIGHT_CELL = floor(LIGHT_POS);
    let TO_LIGHT_SECTORS = LIGHT_CELL - floor(uv * constants.cells_cnt);
    let GENERAL_DIR = sign(TO_LIGHT_SECTORS);
    let STEPS_CNT = abs(TO_LIGHT_SECTORS);
    let MAX_ITERATIONS = i32(STEPS_CNT.x + STEPS_CNT.y);
    let DOUBLED_STEPS = STEPS_CNT * 2.;
    let RAY_START = uv * constants.cells_cnt;

    // I need this hack to mininmaze false empty segments.
    // Sometimes the traverse algorithm pick wrong direction.
    for (var hack_iteration=0; hack_iteration < 2; hack_iteration++) {
        var e   = STEPS_CNT.y - STEPS_CNT.x;
        var pos = floor(uv * constants.cells_cnt);
        
        for (var i = 0; i < MAX_ITERATIONS; i++) {
            let corners = get_inner_rect_for_cell(pos);
            if (rect_hit(RAY_START, normalize(LIGHT_POS - RAY_START), corners.xy, corners.zw)) {
                return 0.5;
            }

            var compare_with = 1e-5 * f32(hack_iteration);
            if (e < compare_with){
                pos.x += GENERAL_DIR.x;
                e += DOUBLED_STEPS.y;
            } else {
                pos.y += GENERAL_DIR.y;
                e -= DOUBLED_STEPS.x;
            }
        }
    }

    return 1.0;
}

//const SS = 4;
@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord: vec2<i32> = vec2<i32>(workgroupID.xy) * vec2(16) + vec2<i32>(localInvocationID.xy);
    let aspect = constants.shadow_res.xy / constants.shadow_res.yy;
    
    // var clr = 0.;
    // for (var y = -SS/2; y < SS/2; y++) {
    //     for (var x = -SS/2; x < SS/2; x++) {
    //         //let offset = vec2<f32>(x, y)
    //         let uv = vec2<f32>(tex_coord + vec2(x, y)) / constants.resolution;
    //         //clr += clamp(color(fragCoord + vec2(x, y) / float(SS)), 0., 1.);
    //         clr += calculate_shadows(uv);
    //     }
    // }
    // clr /= f32(SS * SS);

    let uv = vec2<f32>(tex_coord.xy) / constants.shadow_res * aspect;
    var clr = calculate_shadows(uv);
    textureStore(t_output, tex_coord, vec4<f32>(clr));
}