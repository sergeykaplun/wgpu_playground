struct CameraUniform {
    view_proj: mat4x4<f32>,
};
struct LightUniform {
    view_proj: mat4x4<f32>,
};
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_pos: vec4<f32>,
    @location(2) shadow_pos: vec3<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> light: LightUniform;
@group(2) @binding(0) var t_shadow: texture_depth_2d;
@group(2) @binding(1) var sampler_shadow: sampler_comparison;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = vec4<f32>(in.position - vec3(0.0, 10.0, 0.0), 1.0);
    out.clip_pos = camera.view_proj * out.world_pos;
    out.normal = in.normal;

    let pos_from_light = light.view_proj * out.world_pos;
    out.shadow_pos = vec3(
        pos_from_light.xy * vec2(0.5, -0.5) + vec2(0.5),
        pos_from_light.z
    );

    return out;
}

// fn fetch_shadow(homogeneous_coords: vec4<f32>) -> f32 {
//     if (homogeneous_coords.w <= 0.0) {
//         return 1.0;
//     }
//     // compensate for the Y-flip difference between the NDC and texture coordinates
//     let flip_correction = vec2<f32>(0.5, -0.5);
//     // compute texture coordinates for shadow lookup
//     let proj_correction = 1.0 / homogeneous_coords.w;
//     let light_local = homogeneous_coords.xy * flip_correction * proj_correction + vec2<f32>(0.5, 0.5);
//     // do the lookup, using HW PCF and comparison
//     let depth_ref = homogeneous_coords.z * proj_correction;
//     return textureSampleCompare(t_shadow, sampler_shadow, light_local, 0, depth_ref);
// }

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Percentage-closer filtering. Sample texels in the region
  // to smooth the result.
  var visibility = 0.0;
  let oneOverShadowDepthTextureSize = 1.0 / 1024.;
  for (var y = -1; y <= 1; y++) {
    for (var x = -1; x <= 1; x++) {
      let offset = vec2<f32>(vec2(x, y)) * oneOverShadowDepthTextureSize;

      visibility += textureSampleCompare(
        t_shadow, sampler_shadow,
        in.shadow_pos.xy + offset, in.shadow_pos.z - 0.007
      );
    }
  }
  visibility /= 9.0;

  //let lambertFactor = max(dot(normalize(scene.lightPos - input.fragPos), input.fragNorm), 0.0);
  //let lightingFactor = min(ambientFactor + visibility * lambertFactor, 1.0);
  //return vec4(lightingFactor * albedo, 1.0);
  return vec4(visibility);
}