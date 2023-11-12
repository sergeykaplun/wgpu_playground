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

struct SpatialLookupItem {
    particle_id: u32,
    cell_key: u32,
}

@group(0) @binding(0) var<uniform> constants: Constants;
@group(1) @binding(0) var<storage, read> particle_data: array<Particle>;
@group(1) @binding(1) var<storage, read_write> spatial_lookup: array<SpatialLookupItem>;
@group(1) @binding(2) var<storage, write> start_indices: array<u32>;

const HIGHEST_U32: u32 = 0xFFFFFFFFu;
@compute @workgroup_size(1, 1, 1)
fn write_spatial_lookup(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let id = workgroupID.x;
    let particle = particle_data[id];
    //let cell_coord = position_to_cell_coord(particle.pos);
    let cell_coord = position_to_cell_coord(particle.predicted_pos);
    let cell_key = cell_key_from_hash(hash(cell_coord));

    spatial_lookup[id] = SpatialLookupItem( id, cell_key );
    start_indices[id] = HIGHEST_U32;
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

@compute @workgroup_size(1, 1, 1)
fn write_start_indices(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
        /*@builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>*/) {
    let id = workgroupID.x;
    let key = spatial_lookup[id].cell_key;
    let keyPrev = select(spatial_lookup[id - 1u].cell_key, HIGHEST_U32, id == 0u);
    if (key != keyPrev) {
        start_indices[key] = id;
    }
}

@compute @workgroup_size(128, 1, 1)
fn sort_pairs(@builtin(local_invocation_id) localInvocationID: vec3<u32>, @builtin(workgroup_id) workgroupID: vec3<u32>,
              @builtin(local_invocation_index) localInvocationIndex: u32, @builtin(global_invocation_id) globalInvocationID: vec3<u32>) {
    let i = globalInvocationID.x;

    let h = i & (constants.group_width - 1u);
    let index_low = h + (constants.group_height + 1u) * (i / constants.group_width);
    let index_high = index_low + select((constants.group_height + 1u) / 2u, constants.group_height - 2u * h, constants.step_index == 0u);

    if (index_high >= constants.particles_count) {
        return;
    }

    let value_low = spatial_lookup[index_low];
    let value_high = spatial_lookup[index_high];

    if (value_low.cell_key > value_high.cell_key) {
        spatial_lookup[index_low] = value_high;
        spatial_lookup[index_high] = value_low;
    }
}