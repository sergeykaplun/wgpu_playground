const CELLS_CNT = 2u;

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

//by iq

fn hash(uv_in: vec2<f32>) -> vec2<f32>{
    let k = vec2(0.3183099, 0.3678794);
    let x = uv_in * k + k.yx;
    return -1.0 + 2.0 * fract(16.0 * k * fract(x.x * x.y * (x.x + x.y)));
}

fn voronoi(uv: vec2<f32>) -> vec3<f32>{
    let n = floor(uv);
    let f = fract(uv);

    var mg: vec2<f32>;
    var mr: vec2<f32>;
    var md = 10.;

    for(var j=-1; j<=1; j++) {
        for(var i=-1; i<=1; i++) {
            let g = vec2(f32(i), f32(j));
            let o = hash(n + g);

            let r = g + o - f;
            let d = dot(r, r);

            if(d < md){
                md = d;
                mr = r;
                mg = g;
            }
            /*
            let neighbour = n + vec2(f32(i), f32(j));
            let neighbour_center = neighbour + hash(neighbour);
            //let to_neighbour_center = neighbour + neighbour_center - f;
            //let distance_to_neightbour = dot(to_neighbour_center, to_neighbour_center);
            let distance_to_neightbour = distance(uv, neighbour_center);

            if(distance_to_neightbour < md) {
                md = distance_to_neightbour;
                //mr = to_neighbour_center;
                mg = neighbour_center;
            }
            */
        }
    }
    return vec3(md, mg.x, mg.y);

    /*
    md = 8.0;
    for( int j=-2; j<=2; j++ )
    for( int i=-2; i<=2; i++ )
    {
        vec2 g = mg + vec2(float(i),float(j));
		vec2 o = hash( n + g );
		#ifdef ANIMATE
        o = 0.5 + 0.5*sin( iTime + 6.2831*o );
        #endif	
        vec2 r = g + o - f;

        if( dot(mr-r,mr-r)>0.00001 )
        md = min( md, dot( 0.5*(mr+r), normalize(r-mr) ) );
    }

    return vec3( md, mr );
    */
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let tex_coord = workgroupID.xy * vec2(16u) + localInvocationID.xy;
    let uv = vec2<f32>(f32(tex_coord.x), f32(tex_coord.y)) / constants.resolution;
    let aspect = constants.resolution.xy / constants.resolution.yy;

    let mod_uv = (vec2<f32>(tex_coord.xy) * aspect) % vec2<f32>(vec2<u32>(constants.resolution.xy)/CELLS_CNT);
    let bg = .25 + .25 * step(vec2(1.), mod_uv);
    
    //let beg = vec2<i32>(5);
    //let angle = constants.time;
    //let dxy = vec2<i32>(i32(cos(angle) * 5.0), i32(sin(angle) * 5.0));
    //let col = traverse_along(uv, beg, dxy);
    
    let vor = voronoi(uv * f32(CELLS_CNT));
    textureStore(t_output, tex_coord, vec4<f32>(min(bg.x, bg.y) + smoothstep(.16, .15, vor.x)));
    //textureStore(t_output, tex_coord, vec4<f32>(min(bg.x, bg.y) + vor.x));
}