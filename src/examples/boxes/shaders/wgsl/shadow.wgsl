//const CELLS_CNT = 10u;
//const CELLS_CNT_F = 10.0;

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

/*
fn voxel_clr(uv: vec2<f32>, pos: vec2<i32>) -> f32 {
	let pix = vec2<i32>(floor(uv * f32(CELLS_CNT)));
    return f32(all(pix == pos));
}

fn traverse_along(uv: vec2<f32>, start: vec2<i32>, dir: vec2<i32>) -> f32 {
	var res = 0.0;
	let signs = vec2<i32>(sign(vec2<f32>(dir)));
    let a = abs(dir);
    let b = a * 2;
	var e   = a.y - a.x;
    let max_iterations = (a.x + a.y);
    var pos = start;
    
    for (var i = 0; i < max_iterations; i++) {
		res += voxel_clr(uv, pos);
		
        if (e < 0){
			pos.x += signs.x;
			e += b.y;
		} else {
			pos.y += signs.y;  
			e -= b.x;
		}
    }
	return res;
}

struct Ray {
    origin: vec2<f32>,
    dir: vec2<f32>
};
*/

// source: https://gamedev.stackexchange.com/questions/18436/most-efficient-aabb-vs-ray-collision-algorithms
fn box_hit(raypos: vec2<f32>, raydir: vec2<f32>, boxmin: vec2<f32>, boxmax: vec2<f32>) -> bool {
    let t1 = (boxmin.x - raypos.x) / raydir.x;
    let t2 = (boxmax.x - raypos.x) / raydir.x;
    let t3 = (boxmin.y - raypos.y) / raydir.y;
    let t4 = (boxmax.y - raypos.y) / raydir.y;
    
    let tmin = max(min(t1, t2), min(t3, t4));
    let tmax = min(max(t1, t2), max(t3, t4));

    if (tmin < 0.0 || tmax < 0.0 || tmin > tmax) {
    //if (tmax < 0.0 || tmin > tmax) {
        return false;
    }
    return true;
}

/*
fn calculate_shadows2(uv: vec2<f32>) -> f32 {
    let start_pos = vec2(0.0);
    let end_pos = vec2(1.0);
    //let direction = end_pos - start_pos;
    let direction = vec2(350.0, 350.0);

    //let mod_uv = floor(uv * constants.cells_cnt);
    //let LIGHT_POS = constants.light_position * constants.cells_cnt;
    //let direction = normalize(constants.light_position - uv);

    let pixelSize = vec2(1.0) / 500.;
    var rayPosition = start_pos;
    let rayStep = sign(direction) * pixelSize / abs(direction);
    var pixelCount = 0;
    var res = 0.0;
    
    // if true {
    //     if (all(floor(uv * constants.cells_cnt) == vec2(0.))){
    //         return 1.0;
    //     }
    // }

    //while (pixelCount < 10) {
    for (var i=0; i<10; i++) {
        // let left_up = rayPosition + 0.5 - .125;// + pos_offset;
        // let right_bottom = rayPosition + 0.5 + .125;// + pos_offset;
        // if (box_hit(uv, normalize(LIGHT_POS - uv), left_up, right_bottom)) {
        //     return 0.1;
        // }
        if (all(floor(uv * 500.) == floor(rayPosition))){
            res += 0.2;
        }
        
        rayPosition += rayStep;
        //if (rayPosition.x < 0.0 || rayPosition.y < 0.0 || rayPosition.x > 1.0 || rayPosition.y > 1.0) {
        //    break;
        //}
        pixelCount++;
    }
    return res;
}
*/
fn calculate_shadows(uv: vec2<f32>) -> f32{
    let light_pos = constants.light_position;
    
    { // traverse along light direction
        let LIGHT_CELL = vec2<i32>(floor(light_pos * constants.cells_cnt));
        var TO_LIGHT_SECTORS = LIGHT_CELL - vec2<i32>(floor(uv * constants.cells_cnt));
            
        let GENERAL_DIR = sign(vec2<f32>(TO_LIGHT_SECTORS));
        var STEPS_CNT = abs(TO_LIGHT_SECTORS);
        let MAX_ITERATIONS = STEPS_CNT.x + STEPS_CNT.y;

        let b = STEPS_CNT * 2;
        var e   = STEPS_CNT.y - STEPS_CNT.x;
        var pos = floor(uv * constants.cells_cnt);
        
        for (var i = 0; i < MAX_ITERATIONS; i++) {
            {
                let LIGHT_POS = light_pos * constants.cells_cnt;
                let BIG_UV = uv * constants.cells_cnt;
                let circle = vec3(pos + 0.5, .1);
                
                let index = u32(pos.x * constants.cells_cnt.x + pos.y);
                let pos_offset = cells_centers[index].zw * .25;
                let left_up = pos + 0.5 - .125 + pos_offset;
                let right_bottom = pos + 0.5 + .125 + pos_offset;

                if (box_hit(BIG_UV, normalize(LIGHT_POS - BIG_UV), left_up, right_bottom)) {
                   return 0.5;
                }
            }
            
            if (e < 0){
                pos.x += GENERAL_DIR.x;
                e += b.y;
            } else {
                pos.y += GENERAL_DIR.y;  
                e -= b.x;
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
    
    //let mod_uv = (vec2<f32>(tex_coord.xy) * aspect) % vec2<f32>(vec2<u32>(constants.resolution.xy)/CELLS_CNT);
    //let bg = step(mod_uv, vec2(1.));
    
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

    //let iiii = vec2(6, 4);
    //let llll = vec2<i32>(floor(constants.light_position * CELLS_CNT_F));
    //clr += traverse_along(uv, llll, iiii - llll) * 0.75;

    textureStore(t_output, tex_coord, vec4<f32>(clr));
}