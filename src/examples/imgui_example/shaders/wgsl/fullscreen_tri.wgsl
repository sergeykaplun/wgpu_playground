struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    out.clip_pos = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

struct CameraUniform {
    resolution: vec4<u32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

fn sdBox(p: vec2<f32>, b: vec2<f32>) -> f32{
    let d: vec2<f32> = abs(p)-b;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn ndot(a: vec2<f32>, b: vec2<f32>) -> f32{
    return a.x*b.x - a.y*b.y;
}
fn sdRhombus(in_p: vec2<f32>, b: vec2<f32>) -> f32{
    let p = abs(in_p);
    let h = clamp(ndot(b - vec2<f32>(2.0) * p, b)/dot(b, b), -1.0, 1.0);
    let d = length(p - 0.5 * b * vec2<f32>(1.0 - h, 1.0 + h));
    return d * sign(p.x*b.y + p.y*b.x - b.x*b.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let AA = 4./f32(camera.resolution.y);
    let PI = 3.141593;
    let TAU = PI * 2.;
    let MAX_FLOAT = 1e10;

    let aspect = vec2<f32>(camera.resolution.xy)/vec2<f32>(camera.resolution.yy);
    let uv = in.uv * aspect;
    let center = aspect * vec2<f32>(.5);
    let barycentric = uv - center;
    let ang = atan2(barycentric.x, -barycentric.y);
    let ang_norm = ang/TAU + .5;
    let polar = vec2<f32>(ang_norm, length(barycentric));

    var uv_mod_5: vec2<f32>;
    var mask = MAX_FLOAT;
    
    { //outer gear
        var outer_gear = .45 + pow(cos(polar.x * PI * 25.), 4.) * .05 - polar.y;
        outer_gear = min(outer_gear, polar.y - .375);
        mask = min(mask, outer_gear);
    }
    
    { //screw holes
        let mod_ang = ang % (TAU * .2) - (TAU * .1);
        uv_mod_5 = vec2<f32>(polar.y * sin(mod_ang), polar.y * cos(mod_ang));
        var screw_holes = -sdRhombus(uv_mod_5 - vec2(0., .5), vec2(.15, .15)) + .035;
        screw_holes = min(screw_holes, .45 - polar.y);
        mask = max(mask, screw_holes);
    }
    
    { // R
        var r = -sdBox(barycentric + vec2(.25, .16), vec2(.25, .05)) + .015;
        let vb = -sdBox(barycentric + vec2(.15, 0.), vec2(.08, .2)) + .015;
        let tb = -sdBox(barycentric + vec2(.2, -.2), vec2(.35, .05)) + .015;
        let rr = -distance(barycentric, vec2(.134, .1355)) + .13;
        
        r = max(r, vb);
        r = max(r, tb);
        r = max(r, rr);
        r = min(r, .45 - polar.y);
        mask = max(mask, r);
    }
    
    { //tail
        var tail = barycentric.y + .221 - (1. - pow(smoothstep(.0, .1, barycentric.x), 2.)) * .19;
        tail = min(tail, -barycentric.y - .175 + (1. - pow(smoothstep(0.1, .25, barycentric.x), 2.)) * .2);
        
        tail = max(tail, -sdBox(barycentric - vec2<f32>(.42, -.1), vec2<f32>(.1, .075)) + .02);
        tail = max(tail, -sdBox(barycentric - vec2<f32>(.35, -.15), vec2<f32>(.2, .05)) + .02);
        
        tail = min(tail, .45 - polar.y);
        tail = min(tail, barycentric.x + .1);
        tail = min(tail, distance(barycentric, vec2<f32>(.251, -.07)) - .05);
    
        mask = max(mask, tail);
    }
    mask = max(mask, -sdBox(barycentric - vec2<f32>(.01, .1), vec2<f32>(.135, .15)));
    mask = min(mask, distance(uv_mod_5, vec2<f32>(0., .385)) - .03);
    mask = min(mask, max(-barycentric.x + .05, distance(barycentric, vec2<f32>(0.1, .11)) - .03));
    mask = min(mask, sdBox(barycentric - vec2<f32>(0.02, .11), vec2<f32>(.075, .03)));
    
    return vec4<f32>(smoothstep(0., AA, mask));
}