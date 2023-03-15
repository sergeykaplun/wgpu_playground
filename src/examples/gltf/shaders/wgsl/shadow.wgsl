struct LightUniform {
    view_proj: mat4x4<f32>,
    position: vec4<f32>
};

@group(0) @binding(0) var<uniform> light: LightUniform;

@vertex
fn shadow(@location(0) position: vec3<f32>,) -> @builtin(position) vec4<f32> {
    return light.view_proj * vec4<f32>(position, 1.0);
}