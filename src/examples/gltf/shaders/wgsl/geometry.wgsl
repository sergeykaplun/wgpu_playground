struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec4<f32>,
};
struct LightUniform {
    view_proj: mat4x4<f32>,
    position: vec4<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec3<f32>,
    @location(3) to_light: vec3<f32>,
    @location(4) to_camera: vec3<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> light: LightUniform;
@group(2) @binding(0) var<uniform> node_matrix: mat4x4<f32>;
@group(3) @binding(0) var t_diffuse_tex: texture_2d<f32>;
@group(3) @binding(1) var s_diffuse_tex: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = node_matrix * vec4<f32>(in.position, 1.0);
    out.clip_pos = camera.view_proj * world_pos;
    out.normal = in.normal;
    out.color = in.color;
	out.uv = in.uv;

    out.to_light = light.position.xyz - world_pos.xyz;
	out.to_camera = camera.position.xyz - world_pos.xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse_tex, s_diffuse_tex, in.uv) * vec4<f32>(in.color, 1.0);
    let N = normalize(in.normal);
    let L = normalize(in.to_light);
    let V = normalize(in.to_camera);
    let R = reflect(L, N);
    let diffuse = max(dot(N, L), 0.15) * in.color;
    let specular = pow(max(dot(R, V), 0.0), 16.0) * vec3(0.75);
    return vec4(diffuse * color.rgb + specular, 1.0);
}