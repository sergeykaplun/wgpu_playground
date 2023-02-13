const CELLS_CNT = 10u;

struct Constants
{
    resolution: vec2<f32>,
    time:       f32,
    unused:     f32
};
 
@group(0) @binding(0) var<uniform> constants: Constants;
@group(0) @binding(1) var t_output: texture_storage_2d<r32float, write>;

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

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord = workgroupID.xy * vec2(16u) + localInvocationID.xy;
    let uv = vec2<f32>(f32(tex_coord.x), f32(tex_coord.y)) / constants.resolution;
    let aspect = constants.resolution.xy / constants.resolution.yy;

    let mod_uv = (vec2<f32>(tex_coord.xy) * aspect) % vec2<f32>(vec2<u32>(constants.resolution.xy)/CELLS_CNT);
    let bg = .25 + .25 * step(vec2(2.), mod_uv);
    
    let beg = vec2<i32>(5);
    let angle = constants.time;
    let dxy = vec2<i32>(i32(cos(angle) * 5.0), i32(sin(angle) * 5.0));
    let col = traverse_along(uv, beg, dxy);
    
    textureStore(t_output, tex_coord, vec4<f32>(min(bg.x, bg.y) + col));
}