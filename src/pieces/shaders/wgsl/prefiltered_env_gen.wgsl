struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec4<f32>
};

@group(0) @binding(0) var t_skybox: texture_cube<f32>;
@group(0) @binding(1) var s_skybox: sampler;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;
@group(2) @binding(0) var<uniform> roughness: vec4<f32>;

const NUM_SAMPLES: u32 = 32u;
const PI : f32 = 3.1415926536;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(position, 1.0);
    out.world_pos = model_matrix * out.clip_pos;
    return out;
}

// Based omn http://byteblacksmith.com/improvements-to-the-canonical-one-liner-glsl-rand-for-opengl-es-2-0/
fn random(co: vec2<f32>) -> f32 {
    let a = 12.9898;
    let b = 78.233;
    let c = 43758.5453;
    let dt = dot(co.xy ,vec2(a,b));
    let sn = dt % 3.14;
    return fract(sin(sn) * c);
}

fn hammersley2d(i: u32, N: u32) -> vec2<f32> {
    // Radical inverse based on http://holger.dammertz.org/stuff/notes_HammersleyOnHemisphere.html
    var bits : u32 = (i << 16u) | (i >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    let rdi = f32(bits) * 2.3283064365386963e-10;
    return vec2(f32(i) /f32(N), rdi);
}

// Based on http://blog.selfshadow.com/publications/s2013-shading-course/karis/s2013_pbs_epic_slides.pdf
fn importanceSample_GGX(Xi: vec2<f32>, roughness: f32, normal: vec3<f32>) -> vec3<f32> {
    // Maps a 2D point to a hemisphere with spread based on roughness
    let alpha = roughness * roughness;
    let phi = 2.0 * PI * Xi.x + random(normal.xz) * 0.1;
    let cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (alpha*alpha - 1.0) * Xi.y));
    let sinTheta = sqrt(1.0 - cosTheta * cosTheta);
    let H = vec3(sinTheta * cos(phi), sinTheta * sin(phi), cosTheta);

    // Tangent space
    //TODO double check this
    let up = select(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0), abs(normal.z) < 0.999);
    let tangentX = normalize(cross(up, normal));
    let tangentY = normalize(cross(normal, tangentX));

    // Convert to world Space
    return normalize(tangentX * H.x + tangentY * H.y + normal * H.z);
}

// Normal Distribution function
fn D_GGX(dotNH: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denom = dotNH * dotNH * (alpha2 - 1.0) + 1.0;
    return (alpha2)/(PI * denom*denom);
}

fn prefilterEnvMap(R: vec3<f32>, roughness: f32) -> vec3<f32> {
    let N = R;
    let V = R;
    var color = vec3(0.0);
    var totalWeight = 0.0;
    let envMapDim = f32(textureDimensions(t_skybox, 0).x);
    for (var i: u32 = 0u; i < NUM_SAMPLES; i = i + 1u) {
        let Xi = hammersley2d(i, NUM_SAMPLES);
        let H = importanceSample_GGX(Xi, roughness, N);
        let L = 2.0 * dot(V, H) * H - V;
        let dotNL = clamp(dot(N, L), 0.0, 1.0);
        if(dotNL > 0.0) {
            // Filtering based on https://placeholderart.wordpress.com/2015/07/28/implementation-notes-runtime-environment-map-filtering-for-image-based-lighting/
            let dotNH = clamp(dot(N, H), 0.0, 1.0);
            let dotVH = clamp(dot(V, H), 0.0, 1.0);
            // Probability Distribution Function
            let pdf = D_GGX(dotNH, roughness) * dotNH / (4.0 * dotVH) + 0.0001;
            // Slid angle of current smple
            let omegaS = 1.0 / (f32(NUM_SAMPLES) * pdf);
            // Solid angle of 1 pixel across all cube faces
            let omegaP = 4.0 * PI / (6.0 * envMapDim * envMapDim);
            // Biased (+1.0) mip level for better result
            //TODO double check this
            let mipLevel = select(max(0.5 * log2(omegaS / omegaP) + 1.0, 0.0), 0.0, roughness == 0.0);
            color = color + textureSampleLevel(t_skybox, s_skybox, L, mipLevel).rgb * dotNL;
            totalWeight = totalWeight + dotNL;
        }
    }
    return color / totalWeight;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var to_fragment = normalize(in.world_pos.xyz);
    return vec4(prefilterEnvMap(to_fragment, roughness.x), 1.0);
}