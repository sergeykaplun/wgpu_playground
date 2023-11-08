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

    target_density: f32,// = 20.75;
    pressure_multiplier: f32,// = 0.5;
    pointer_location: vec2<f32>,

    resolution: vec2<f32>,
    pointer_active: f32,
    pointer_attract: f32,

    group_width: u32,
    group_height: u32,
    step_index: u32,
    _padding2: u32,
};

@group(0) @binding(0) var<uniform> constants: Constants;
@group(1) @binding(0) var<storage, read> particle_data: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(f32((in_vertex_index << 1u) & 2u), f32(in_vertex_index & 2u));
    out.clip_pos = vec4<f32>(out.uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = (out.uv * 2.0 - 1.0) * vec2<f32>(constants.aspect, 1.0) * 5.;
    return out;
}

fn bound(uv: vec2<f32>) -> f32 {
    let width = constants.particle_radius * 0.25;
    let half_bounds = constants.bounds_size * 0.5 + width;
    let mask = max(step(distance(abs(uv.x), half_bounds.x), width) * step(abs(uv.y), half_bounds.y + width),
                   step(distance(abs(uv.y), half_bounds.y), width) * step(abs(uv.x), half_bounds.x + width));
    return mask;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (true)
    {
        var bounds = mix(vec3(0.0), vec3<f32>(0.8, 0.125, 0.2), bound(in.uv));
        if (constants.pointer_active > 0.0)
        {
            let pl = (constants.pointer_location/constants.resolution * 2.0 - 1.0) * 5.0 * vec2(constants.aspect, -1.0);
            bounds = mix(bounds, vec3(0.0, 1.0, 0.0), smoothstep(0.05, 0.025, distance(distance(in.uv, pl), 2.0)));
        }
        return vec4(bounds, 1.0);
    }

    //var clr = vec3<f32>(0.05, 0.2, 1.) * calc_density(in.uv) * 0.025;
    //let pressure = convert_density_to_pressure(calc_density(in.uv));
    let dens = calc_density(in.uv) - constants.target_density;
    var clr = mix(vec3(1.0),
                  mix(vec3(0.0, 0.0, 1.0),
                      vec3(1.0, 0.0, 0.0),
                      step(0.0, sign(dens))),
                  smoothstep(0., 2., abs(dens)));
                  // * pressure * 50.;
    //clr = mix(clr, vec3<f32>(0.8, 0.125, 0.2), bound(in.uv));
    //return vec4(clr, 1.0);


    if(false){
        //TODO fix
        let cells_cnt = 2.;
        let cell_span = 1./cells_cnt;
        var cell_uv = fract(in.uv * cells_cnt);

        let cell_cntr = in.uv - (in.uv % cell_span) + cell_span * 0.5;
        let grad = calc_density_gradient(cell_cntr);
        let ang = atan2(grad.y, grad.x);
        let cos_rotation = cos(ang);
        let sin_rotation = sin(ang);
        let rotation_matrix = mat2x2(cos_rotation, -sin_rotation, sin_rotation, cos_rotation);

        cell_uv -= 0.5;
        cell_uv *= clamp(smoothstep(4.0, 0.0, abs(calc_density(cell_cntr) - constants.target_density)) * 2.0, 0.01, 2.0);
        cell_uv *= rotation_matrix;
        cell_uv += 0.5;

        clr = mix(clr, vec3(0.0), arrow_mask(cell_uv));
    }
    //clr = mix(clr, vec3<f32>(0.8, 0.125, 0.2), bound(in.uv));
    return vec4<f32>(clr, 1.);

}

fn arrow_mask(uv: vec2<f32>) -> f32{
    let mirrored = abs(uv - 0.5) * 2.0;
    let a = step(mirrored.x, .275 - abs(mirrored.y)) * step(.225 - abs(mirrored.y), mirrored.x)
          * step(abs(mirrored.y), mirrored.x + 0.15) * step(0.5, uv.x);
    let b = step(abs(mirrored.x), 0.25) * step(abs(mirrored.y), 0.02);

    return max(a, b);
}

const PI: f32 = 3.1415926535897932384626433832795;
/*
fn smooth_kernel(radius: f32, dst: f32) -> f32{
    let volume = PI * pow(radius, 8.0) / 4.0;
    let value = max(0.0, radius * radius - dst * dst);
    return value * value * value / volume;
}
*/

fn smooth_kernel(radius: f32, dst: f32) -> f32{
    if(dst >= radius) { return 0.0; }

    let volume = PI * pow(radius, 4.0) / 6.0;
    return (radius - dst) * (radius - dst) / volume;
}

fn smooth_kernel_derivative(dst: f32, rad: f32) -> f32 {
    if (dst >= rad) { return 0.0; }
    let scale = 12. / (PI * pow(rad, 4.0));
    return scale * (dst - rad);
}

fn calc_density(sample_point: vec2<f32>) -> f32 {
    var density = 0.0;

    for (var i = 0u; i < constants.particles_count; i = i + 1u) {
        let particle = particle_data[i];
        let dist = distance(sample_point, particle.pos);
        let influence = smooth_kernel(constants.smoothing_radius, dist);

        density += constants.particle_mass * influence;
    }
    return density;
}
/*
fn smooth_kernel_derivative(dst: f32, rad: f32) -> f32 {
    if (dst > rad) { return 0.0; }
    let f = rad * rad - dst * dst;
    let scale = -24.0 / (PI * pow(rad, 8.0));
    return scale * dst * f * f;
}
*/

fn convert_density_to_pressure(density: f32) -> f32 {
    let density_error = density - constants.target_density;
    let pressure = density_error * constants.pressure_multiplier;
    return pressure;
}

fn calc_pressure_force(sample_point: vec2<f32>) -> vec2<f32> {
    var pressure_force = vec2<f32>(0.0);

    for (var i = 0u; i < constants.particles_count; i = i + 1u) {
        let particle = particle_data[i];
        let dist = distance(sample_point, particle.pos);
        let dir = (particle.pos - sample_point)/dist;
        let slope = smooth_kernel_derivative(dist, constants.smoothing_radius);
        let density = particle.density;

        pressure_force += -convert_density_to_pressure(density) * dir * slope * constants.particle_mass / density;
    }
    return pressure_force;
}

fn calc_density_gradient(sample_point: vec2<f32>) -> vec2<f32> {
    var density_gradient = vec2<f32>(0.0);

    for (var i = 0u; i < constants.particles_count; i = i + 1u) {
        let particle = particle_data[i];
        let dist = distance(sample_point, particle.pos);
        let dir = (particle.pos - sample_point)/dist;
        let slope = smooth_kernel_derivative(dist, constants.smoothing_radius);
        let density = particle.density;

        //density_gradient += /*-particleProperties[i]*/ dir * slope * mass / density;
        density_gradient += dir * slope * constants.particle_mass;
    }
    return density_gradient;
}