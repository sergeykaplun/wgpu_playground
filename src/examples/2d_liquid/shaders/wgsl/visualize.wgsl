const TAU: f32 = 6.283185307179586476925286766559;

struct Particle {
    pos: vec2<f32>,
    predicted_pos: vec2<f32>,
    vel: vec2<f32>,
    density: f32,
    _padding: f32,
};

struct Constants {
    gravity: vec2<f32>,
    smoothing_radius: f32,
    particle_mass: f32,

    aspect: f32,
    particle_segments: u32,
    particle_radius: f32,
    delta_time: f32,

    bounds_size: vec2<f32>,
    damping: f32,
    particles_count: u32,

    target_density: f32,
    pressure_multiplier: f32,
    pointer_location: vec2<f32>,

    resolution: vec2<f32>,
    pointer_active: f32,
    pointer_attract: f32,
};

@group(0) @binding(0) var<uniform> constants: Constants;
@group(1) @binding(0) var<storage, read> particle_data: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) clr: vec3<f32>,
    @location(1) dist: f32,
};

fn palette(h: f32) -> vec3<f32> {
    var col =    vec3(0.0,0.3,1.0);
    col = mix(col, vec3(1.0,0.8,0.0), smoothstep(0.13, 0.53, h));
    col = mix(col, vec3(1.0,0.0,0.0), smoothstep(0.46, 0.86, h));
    col.y += 0.5*(1.0-smoothstep(0.0, 0.2, abs(h - f32(0.33))));
    col *= 0.5 + 0.5*h;
    return col;
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    //TODO reduce uniform computations
    let SEGMENT_VERTICES = constants.particle_segments * 3u;
    let particle_id = in_vertex_index / SEGMENT_VERTICES;
    let segment_id = (in_vertex_index % SEGMENT_VERTICES) / 3u;
    let vertex_id = (in_vertex_index % SEGMENT_VERTICES) % 3u;

    //let SEGMENT_SPAN = TAU / f32(constants.particle_segments);
    let SEGMENT_SPAN = TAU / 24.0;

    let particle_pos: vec2<f32> = particle_data[particle_id].pos;// - vec2(8., 4.5);
    var offset = vec2<f32>(0.0, 0.0);
    switch (vertex_id) {
        case 0u: {
            offset = vec2<f32>(0.0, 0.0);
            out.dist = 0.0;
        }
        case 1u: {
            offset = vec2<f32>(constants.particle_radius * cos((f32(segment_id) + 1.0) * SEGMENT_SPAN), constants.particle_radius * sin((f32(segment_id) + 1.0) * SEGMENT_SPAN));
            out.dist = constants.particle_radius;
        }
        case 2u: {
            offset = vec2<f32>(constants.particle_radius * cos(f32(segment_id) * SEGMENT_SPAN), constants.particle_radius * sin(f32(segment_id) * SEGMENT_SPAN));
            out.dist = constants.particle_radius;
        }
        default: {
            offset = vec2<f32>(0.0, 0.0);
        }
    }
    //offset /= vec2(constants.aspect, 1.0);

    out.clip_pos = vec4<f32>((particle_pos + offset)/vec2(constants.aspect, 1.0) * 0.2, 0.0, 1.0);
    let speed = length(particle_data[particle_id].vel);
    out.clr = palette(smoothstep(0., 2.5, speed));
    //out.clr = vec3<f32>(1.0, 1.0, 0.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var clr = in.clr;
    return vec4<f32>(clr * step(in.dist, constants.particle_radius * 0.75), 1.0);
}