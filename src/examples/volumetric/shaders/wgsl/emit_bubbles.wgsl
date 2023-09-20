struct PushConstants {
    iTime: f32,
}
@group(0) @binding(0) var t_output: texture_storage_3d<rgba16float, write>;
@group(1) @binding(0) var<uniform> constants : PushConstants;

fn hash2(p: vec2<f32>) -> vec2<f32> {
    // A simple hash function using dot products and modulo
    let p2d: vec2<f32> = vec2<f32>(127.1, 311.7);
    let dot_product: f32 = dot(p, p2d);
    let fractional: f32 = fract(sin(dot_product) * 43758.5453);

    return vec2<f32>(fract(fractional), fract(fractional * 1.3));
}

fn voronoi(x: vec2<f32>) -> vec3<f32> {
    let n = floor(x);
    let f = fract(x);

    var mg: vec2<f32>;
    var mr: vec2<f32>;
    var target_cntr: vec2<f32>;

    var max_distance: f32 = 8.0;
    for (var j: i32 = -1; j <= 1; j = j + 1) {
        for (var i: i32 = -1; i <= 1; i = i + 1) {
            let to_neightbour: vec2<f32> = vec2<f32>(f32(i), f32(j));
            var center_inside_neighbour: vec2<f32> = hash2(n + to_neightbour);

//            #ifdef ANIMATE
            let time: f32 = constants.iTime;
            center_inside_neighbour = 0.5 + 0.5 * sin(6.2831 * center_inside_neighbour + time);
//            #endif

            let r: vec2<f32> = to_neightbour + center_inside_neighbour - f;
            let d: f32 = dot(r, r);

            if (d < max_distance) {
                max_distance = d;
                mr = r;
                mg = to_neightbour;
                target_cntr = n + to_neightbour + center_inside_neighbour;
            }
        }
    }
    return vec3<f32>(max_distance, target_cntr.x, target_cntr.y);

    /*
    //----------------------------------
    // second pass: distance to borders
    //----------------------------------
    max_distance = 8.0;
    for (var j: i32 = -2; j <= 2; j = j + 1) {
        for (var i: i32 = -2; i <= 2; i = i + 1) {
            let g: vec2<f32> = mg + vec2<f32>(f32(i), f32(j));
            var o: vec2<f32> = hash2(n + g);

//            #ifdef ANIMATE
//            let time: f32 = constants.iTime;
//            o = 0.5 + 0.5 * sin(6.2831 * o + time);
//            #endif

            let r: vec2<f32> = g + o - f;

            if (dot(mr - r, mr - r) > 0.00001) {
                max_distance = min(max_distance, dot(0.5 * (mr + r), normalize(r - mr)));
            }
        }
    }
    return vec3<f32>(max_distance, mr.x, mr.y);

    */
}

fn saturate(val: vec3<f32>) -> vec3<f32> {
    return clamp(val, vec3(0.), vec3(1.));
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let rgb = saturate(abs((c.x * 6.0 + vec3(0.0, 4., 2.) % 6.) - 3.) - 1.);
    return c.z * mix(vec3(1.), rgb, c.y);
}

@compute @workgroup_size(4, 4, 4)
fn emit(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let tex_coord: vec3<i32> = vec3<i32>(workgroupID) * vec3(4) + vec3<i32>(localInvocationID);
    let uv = vec3<f32>(tex_coord) / vec3<f32>(512.);
    let TILES = 5.;
    let scaled_uv = uv * TILES;
    let vor = voronoi(scaled_uv.xz);
    //let cntr_mask = step(length(vec3<f32>(vor.y, (0.5 - uv.y) * TILES, vor.z)), 0.1);
    let cntr_mask = step(distance(scaled_uv, vec3<f32>(vor.y, 0.1 * TILES, vor.z)), 0.1);

    let hash = hash2(vor.yz * constants.iTime);
    let mask = cntr_mask * step(0.99, hash.x);
            //* step(0.8, hash2(vor.yz * floor(constants.iTime % 10.) * 0.1).x);
            //* step(floor(constants.iTime % 10.) * 0.1, hash2(vor.yz).x);
    if(mask > 0.0)
    {
        var clr = vec4(hsv2rgb(vec3(hash2(vor.yz).x, 1., 1.)), 1.0);
        textureStore(t_output, tex_coord, vec4(clr));
    }
}