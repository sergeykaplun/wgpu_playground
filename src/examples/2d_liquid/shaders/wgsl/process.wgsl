const PI: f32 = 3.1415926535897932384626433832795;

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
};

struct SpatialLookupItem {
    particle_id: u32,
    cell_key: u32,
}

@group(0) @binding(0) var<uniform> constants: Constants;
@group(1) @binding(0) var<storage, read_write> particle_data: array<Particle>;
@group(1) @binding(1) var<storage, read> spatial_lookup: array<SpatialLookupItem>;
@group(1) @binding(2) var<storage, read> start_indices: array<u32>;

@compute @workgroup_size(256, 1, 1)
fn calculate_predicted_pos(@builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    var particle = particle_data[globalInvocationID.x];
    particle.vel += constants.gravity_strength * constants.gravity * constants.delta_time;
    particle.predicted_pos = particle.pos + particle.vel * 1./120.;
    particle_data[globalInvocationID.x] = particle;
}

@compute @workgroup_size(256, 1, 1)
fn calculate_particle_densities(@builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    var particle = particle_data[globalInvocationID.x];
    particle.density = calc_density(particle.predicted_pos);
    particle_data[globalInvocationID.x] = particle;
}

@compute @workgroup_size(256, 1, 1)
fn apply_particles_pressure(@builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    var particle = particle_data[globalInvocationID.x];
    let pressure_force = calc_pressure_force(globalInvocationID.x);
    let pressure_accel = pressure_force / particle.density;
    particle.vel += pressure_accel * constants.delta_time;
    particle_data[globalInvocationID.x] = particle;
}

@compute @workgroup_size(256, 1, 1)
fn update_particles_positions(@builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    var particle = particle_data[globalInvocationID.x];
    if (constants.pointer_active == 1.0) {
        particle.vel += calculate_interaction_force(particle.pos, particle.vel) * constants.delta_time;
    }
    particle.pos += particle.vel * constants.delta_time;

    let half_bounds = constants.bounds_size * 0.5 - constants.particle_radius;
    if (abs(particle.pos.x) > half_bounds.x) {
        particle.pos.x = sign(particle.pos.x) * half_bounds.x;
        particle.vel.x *= -constants.damping;
    }
    if (abs(particle.pos.y) > half_bounds.y) {
        particle.pos.y = sign(particle.pos.y) * half_bounds.y;
        particle.vel.y *= -constants.damping;
    }

    particle_data[globalInvocationID.x] = particle;
}

fn position_to_cell_coord(pos: vec2<f32>) -> vec2<u32> {
    return vec2<u32>(floor(pos / constants.smoothing_radius));
}

fn hash(coord: vec2<u32>) -> u32 {
    let h = vec2(coord * vec2(12823u, 9737333u));
    return h.x + h.y;
}

fn cell_key_from_hash(hash: u32) -> u32 {
    return hash % constants.particles_count;
}

fn calc_density(sample_point: vec2<f32>) -> f32 {
    var density = 0.0;
    /*
    let cell_coord = vec2<i32>(position_to_cell_coord(sample_point));
    for(var i = -1; i <= 1; i=i+1) {
        for(var j = -1; j <= 1; j=j+1) {
            //if(any(cell_coord < vec2(0)) && any(cell_coord >= vec2<i32>(ceil(constants.bounds_size / constants.smoothing_radius)))) {
            //    continue;
            //}
            let neighbour_cell = vec2<u32>(vec2<i32>(cell_coord) + vec2(i, j));
            let cell_key = cell_key_from_hash(hash(neighbour_cell));
            var cur_index = start_indices[cell_key];
            loop {
                let psd = spatial_lookup[cur_index];
                if (psd.cell_key != cell_key) {
                    break;
                }
                let particle = particle_data[psd.particle_id];
                let dist = distance(sample_point, particle.pos);
                let influence = smooth_kernel(constants.smoothing_radius, dist);
                density += constants.particle_mass * influence;
                cur_index = cur_index + 1u;
            }
        }
    }
    */

    for (var i = 0u; i < constants.particles_count; i = i + 1u) {
        let particle = particle_data[i];
        let dist = distance(sample_point, particle.pos);
        let influence = smooth_kernel(dist);

        density += constants.particle_mass * influence;
    }

    return density;
}

/*
fn smooth_kernel(radius: f32, dst: f32) -> f32 {
    let vol = PI * pow(radius, 8.0) / 4.0;
    let val = max(0.0, radius * radius - dst * dst);
    return val * val * val / vol;
}

fn smooth_kernel_derivative(radius: f32, dst: f32) -> f32 {
    if (dst >= radius) { return 0.0; }
    let f = radius * radius - dst * dst;
    let scale = -24. / (PI * pow(radius, 8.0));
    return scale * dst * f * f;
}
*/

fn smooth_kernel_rad(dst: f32, radius: f32) -> f32{
    if(dst >= radius) { return 0.0; }

    let volume = PI * pow(radius, 4.0) / 6.0;
    return (radius - dst) * (radius - dst) / volume;
}

fn smooth_kernel(dst: f32) -> f32{
    return smooth_kernel_rad(dst, constants.smoothing_radius);
}

fn smooth_kernel_derivative(dst: f32) -> f32 {
    let radius = constants.smoothing_radius;
    if (dst >= radius) { return 0.0; }
    let scale = 12. / (PI * pow(radius, 4.0));
    return scale * (dst - radius);
}

fn convert_density_to_pressure(density: f32) -> f32 {
    let density_error = density - constants.target_density;
    let pressure = density_error * constants.pressure_multiplier;
    return pressure;
}

fn calc_pressure_force(particle_index: u32) -> vec2<f32> {
    var pressure_force = vec2<f32>(0.0);
    let cur_particle = particle_data[particle_index];
    for (var other_particle_index = 0u; other_particle_index < constants.particles_count; other_particle_index = other_particle_index + 1u) {
        if (particle_index == other_particle_index) { continue; }
        let other_particle = particle_data[other_particle_index];
        //let offset = other_particle.pos - cur_particle.predicted_pos;
        let offset = other_particle.predicted_pos - cur_particle.predicted_pos;
        let dist = length(offset);
        var dir = vec2(0.0);
        if (dist > 0.0) {
            dir = offset / dist;
        }

        let slope = smooth_kernel_derivative(dist);
        let density = other_particle.density;
        let shared_pressure = (convert_density_to_pressure(density) + convert_density_to_pressure(cur_particle.density)) * 0.5;
        pressure_force += shared_pressure * dir * slope * constants.particle_mass / density;
    }

    return pressure_force;
}

fn pointer_location() -> vec2<f32>{
    return (constants.pointer_location/constants.resolution * 2.0 - 1.0) * 5.0 * vec2(constants.aspect, -1.0);
}

fn calculate_interaction_force(pos: vec2<f32>, vel: vec2<f32>) -> vec2<f32> {
    var force = vec2<f32>(0.0);
    var offset = pointer_location() - pos;
    if (constants.pointer_attract > 0.0) {
        offset *= -1.0;
    }
    let dist = length(offset);

    let radius = 3.0;
    let strength = constants.pressure_multiplier * 2.0;
    if(dist < radius) {
        let dir_to_pointer = normalize(offset);
        let cntr = 1.0 - dist / radius;
        force += (dir_to_pointer * strength - vel) * cntr;
    }

    return force;
}

/*
fn calculate_interaction_force(pos: vec2<f32>, vel: vec2<f32>) -> vec2<f32> {
    var offset = pointer_location() - pos;
    if (constants.pointer_attract > 0.0) {
        offset *= -1.0;
    }
    let dist = length(offset);
    if (dist > 3.0)
    {
        return vec2(0.0);
    }
    let strength = smooth_kernel_rad(dist, 3.0) * constants.pressure_multiplier * 10.;
    return (normalize(-offset) * strength - vel);
}*/
