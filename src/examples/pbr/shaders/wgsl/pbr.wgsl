// PBR shader based on the Khronos WebGL PBR implementation
// See https://github.com/KhronosGroup/glTF-WebGL-PBR
// Supports metallic roughness inputs

struct VertexInput {
    @location(0) pos :              vec3<f32>,
    @location(1) normal :           vec3<f32>,
    @location(2) uv0 :              vec2<f32>,
    @location(3) uv1 :              vec2<f32>,
    @location(4) color :            vec3<f32>,
    //location(4) inJoint0 :    vec4<u32>;
    //location(5) inWeight0 :   vec4<f32>;
};

struct CameraParams {
  projection :                      mat4x4<f32>,
  model :                           mat4x4<f32>,
  view :                            mat4x4<f32>,
  position :                        vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera_params : CameraParams;
//group(1) for pbr textures set

struct UBONode {
  transform :                          mat4x4<f32>,
  //jointMatrix : [[stride(64)]] array<mat4x4<f32>, MAX_NUM_JOINTS>;
  //jointCount : f32;
};
@group(2) @binding(0) var<uniform> node : UBONode;

struct LightingParams {
	light_dir:                      vec4<f32>,
	exposure:                       f32,
	gamma:                          f32,
	prefiltered_cube_mip_levels:    f32,
	scale_ibl_ambient:              f32,
	//debug_view_inputs:              float,
	//debug_view_equation:            float,
};
@group(3) @binding(0) var<uniform> lighting_params : LightingParams;
@group(3) @binding(1) var t_irradiance: texture_cube<f32>;
@group(3) @binding(2) var s_irradiance: sampler;
@group(3) @binding(3) var t_prefiltered_env: texture_cube<f32>;
@group(3) @binding(4) var s_prefiltered_env: sampler;
@group(3) @binding(5) var t_brdf_lut: texture_2d<f32>;
@group(3) @binding(6) var s_brdf_lut: sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) color: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = in.color;

    let locPos = camera_params.model * node.transform * vec4(in.pos, 1.0);
    //out.normal = normalize(transpose(inverse(mat3(camera_params.model * node.matrix))) * in.normal);
    out.normal = normalize(camera_params.model * node.transform * vec4(in.normal, 1.0)).xyz;

    //locPos.y = -locPos.y;
	out.world_pos = locPos.xyz / locPos.w;
	out.uv0 = in.uv0;
    out.uv1 = in.uv1;
    out.clip_pos = camera_params.projection * camera_params.view * vec4(out.world_pos, 1.0);
    //out.clip_pos = camera_params.view_proj * vec4(out.world_pos, 1.0);
    
    return out;
}

@group(1) @binding(0) var t_color_map: texture_2d<f32>;
@group(1) @binding(1) var s_color_map: sampler;
@group(1) @binding(2) var t_physical_distribution_map: texture_2d<f32>;
@group(1) @binding(3) var s_physical_distribution_map: sampler;
@group(1) @binding(4) var t_normal_map: texture_2d<f32>;
@group(1) @binding(5) var s_normal_map: sampler;
@group(1) @binding(6) var t_ao_map: texture_2d<f32>;
@group(1) @binding(7) var s_ao_map: sampler;
@group(1) @binding(8) var t_emissive_map: texture_2d<f32>;
@group(1) @binding(9) var s_emissive_map: sampler;
@group(1) @binding(10) var<uniform> material : Material;

struct Material {
	base_color_factor:                      vec4<f32>,
    // vec4 emissive_factor;
	// vec4 diffuse_factor;
	// vec4 specularFactor;
	// //float workflow;
    base_color_texture_set:                 i32,
	physical_descriptor_texture_set:        i32,
	normal_texture_set:                     i32,
	occlusion_texture_set:                  i32,
	emissive_texture_set:                   i32,
	metallic_factor:                        f32,
	roughness_factor:                       f32,
	alpha_mask:                             f32,
	alpha_mask_cutoff:                      f32,
	alignment:                              vec3<f32>,
};

const MIN_ROUGHNESS : f32 = 0.04;
const M_PI : f32 = 3.141592653589793;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    /*if (true) {
        let nrm = get_normal(in);
        return vec4(nrm, 1.0);
    }*/

    var perceptual_roughness: f32;
	var metallic: f32;
	var diffuse_color: vec3<f32>;
	var base_color: vec4<f32>;
	var should_discard = false;

    let f0 = vec3(0.04);

    if (material.alpha_mask == 1.0f) {
		base_color = material.base_color_factor;
        if (material.base_color_texture_set > -1) {
            base_color *= SRGBtoLINEAR(textureSample(t_color_map, s_color_map, select(in.uv1, in.uv0, material.base_color_texture_set == 0)));
        }
        if (base_color.a < material.alpha_mask_cutoff) {
            should_discard = true;
        }
	}

	{// workflow == PBR_WORKFLOW_METALLIC_ROUGHNESS
	    perceptual_roughness = material.roughness_factor;
	    metallic = material.metallic_factor;
	    if (material.physical_descriptor_texture_set > -1) {
            let mrSample = textureSample(t_physical_distribution_map, s_physical_distribution_map, select(in.uv1, in.uv0, material.physical_descriptor_texture_set == 0));
            perceptual_roughness *= mrSample.g;
            metallic *= mrSample.b;
        } else {
            perceptual_roughness = clamp(perceptual_roughness, MIN_ROUGHNESS, 1.0);
            metallic = clamp(metallic, 0.0, 1.0);
        }

        base_color = material.base_color_factor;
        if (material.base_color_texture_set > -1) {
            base_color *= SRGBtoLINEAR(textureSample(t_color_map, s_color_map, select(in.uv1, in.uv0, material.base_color_texture_set == 0)));
        }
	}

    // TODO process PBR_WORKFLOW_SPECULAR_GLOSINESS

    base_color *= vec4(in.color, 1.0);
    diffuse_color = base_color.rgb * (vec3(1.0) - f0) * (1.0 - metallic);

    let alpha_roughness = perceptual_roughness * perceptual_roughness;
    let specular_color = mix(f0, base_color.rgb, metallic);
    let reflectance = max(max(specular_color.r, specular_color.g), specular_color.b);
    let reflectance_90 = clamp(reflectance * 25.0, 0.0, 1.0);
    let specular_environment_R0 = specular_color.rgb;
    let specular_environment_R90 = vec3(1.0, 1.0, 1.0) * reflectance_90;

    let n = select(normalize(in.normal), get_normal(in), material.normal_texture_set > -1);
    let v = normalize(camera_params.position.xyz - in.world_pos);
    //let l = normalize(light_params.light_pos - in.world_pos);
    let l = normalize(-lighting_params.light_dir.xyz);
    let h = normalize(v + l);
    var reflection = -normalize(reflect(v, n));
    reflection.y *= -1.0;

    let NdotL = clamp(dot(n, l), 0.001, 1.0);
    let NdotV = clamp(abs(dot(n, v)), 0.001, 1.0);
    let NdotH = clamp(dot(n, h), 0.0, 1.0);
    let LdotH = clamp(dot(l, h), 0.0, 1.0);
    let VdotH = clamp(dot(v, h), 0.0, 1.0);

    let pbr_inputs = PBRInfo(
        NdotL, NdotV, NdotH, LdotH, VdotH, perceptual_roughness, metallic, specular_environment_R0,
        specular_environment_R90, alpha_roughness, diffuse_color, specular_color
    );

    let F = specular_reflection(pbr_inputs);
    let G = geometric_occlusion(pbr_inputs);
    let D = microfacet_distribution(pbr_inputs);

    let light_color = vec3(1.0);
    let diffuse_contrib = (1.0 - F) * diffuse(pbr_inputs);
    let specular_contrib = F * G * D / (4.0 * NdotL * NdotV);
    var color = NdotL * light_color * (diffuse_contrib + specular_contrib)
              + get_ibl_contribution(pbr_inputs, n, reflection);
    let occlusion_strength = 1.0;
    if (material.occlusion_texture_set > -1) {
        let ao = textureSample(t_ao_map, s_ao_map, select(in.uv1, in.uv0, material.occlusion_texture_set == 0)).r;
        color = mix(color, color * ao, occlusion_strength);
    }

    let emssive_factor = 1.0;
    if (material.emissive_texture_set > -1) {
        let emissive = textureSample(t_emissive_map, s_emissive_map, select(in.uv1, in.uv0, material.emissive_texture_set == 0)).rgb;
        color += emissive * emssive_factor;
    }

    if (should_discard) {
        discard;
    }
    return vec4(color, base_color.a);
}

struct PBRInfo {
	NdotL:                  f32,                  // cos angle between normal and light direction
	NdotV:                  f32,                  // cos angle between normal and view direction
	NdotH:                  f32,                  // cos angle between normal and half vector
	LdotH:                  f32,                  // cos angle between light direction and half vector
	VdotH:                  f32,                  // cos angle between view direction and half vector
	perceptualRoughness:    f32,                  // roughness value, as authored by the model creator (input to shader)
	metalness:              f32,                  // metallic value at the surface
	reflectance0:           vec3<f32>,            // full reflectance color (normal incidence angle)
	reflectance90:          vec3<f32>,            // reflectance color at grazing angle
	alphaRoughness:         f32,                  // roughness mapped to a more linear change in the roughness (proposed by [2])
	diffuseColor:           vec3<f32>,            // color contribution from diffuse lighting
	specularColor:          vec3<f32>,            // color contribution from specular lighting
};

fn SRGBtoLINEAR(srgbIn: vec4<f32>) -> vec4<f32> {
	let bLess = step(vec3(0.04045), srgbIn.xyz);
	let linOut = mix(srgbIn.xyz/vec3(12.92), pow((srgbIn.xyz+vec3(0.055))/vec3(1.055),vec3(2.4)), bLess);

	return vec4(linOut, srgbIn.w);
}

// TODO clarify
fn get_normal(in: VertexOutput) -> vec3<f32> {
    let tangent_normal = textureSample(t_normal_map, s_normal_map, select(in.uv1, in.uv0, material.normal_texture_set == 0)).xyz * 2.0 - 1.0;
    if(true)
    {
        return tangent_normal;
        //return vec3(in.uv0, 0.0);
    }

    let q1 = dpdx(in.world_pos);
    let q2 = dpdy(in.world_pos);
    let st1 = dpdx(in.uv0);
    let st2 = dpdy(in.uv0);

    let N = normalize(in.normal);
    let T = normalize(q1 * st2.y - q2 * st1.y);
    let B = -normalize(cross(N, T));
    let TBN = mat3x3<f32>(T, B, N);

    return normalize(TBN * tangent_normal);
}

// The following equation models the Fresnel reflectance term of the spec equation (aka F())
// Implementation of fresnel from [4], Equation 15
fn specular_reflection(pbr_inputs: PBRInfo) -> vec3<f32> {
	return pbr_inputs.reflectance0 + (pbr_inputs.reflectance90 - pbr_inputs.reflectance0) * pow(clamp(1.0 - pbr_inputs.VdotH, 0.0, 1.0), 5.0);
}

// This calculates the specular geometric attenuation (aka G()),
// where rougher material will reflect less light back to the viewer.
// This implementation is based on [1] Equation 4, and we adopt their modifications to
// alphaRoughness as input as originally proposed in [2].
fn geometric_occlusion(pbr_inputs: PBRInfo) -> f32 {
	let NdotL = pbr_inputs.NdotL;
	let NdotV = pbr_inputs.NdotV;
	let r = pbr_inputs.alphaRoughness;

	let attenuationL = 2.0 * NdotL / (NdotL + sqrt(r * r + (1.0 - r * r) * (NdotL * NdotL)));
	let attenuationV = 2.0 * NdotV / (NdotV + sqrt(r * r + (1.0 - r * r) * (NdotV * NdotV)));
	return attenuationL * attenuationV;
}

// The following equation(s) model the distribution of microfacet normals across the area being drawn (aka D())
// Implementation from "Average Irregularity Representation of a Roughened Surface for Ray Reflection" by T. S. Trowbridge, and K. P. Reitz
// Follows the distribution function recommended in the SIGGRAPH 2013 course notes from EPIC Games [1], Equation 3.
fn microfacet_distribution(pbr_inputs: PBRInfo) -> f32 {
	let roughness_sq = pbr_inputs.alphaRoughness * pbr_inputs.alphaRoughness;
	let f = (pbr_inputs.NdotH * roughness_sq - pbr_inputs.NdotH) * pbr_inputs.NdotH + 1.0;
	return roughness_sq / (M_PI * f * f);
}

// Basic Lambertian diffuse
// Implementation from Lambert's Photometria https://archive.org/details/lambertsphotome00lambgoog
// See also [1], Equation 1
fn diffuse(pbr_inputs: PBRInfo) -> vec3<f32> {
	return pbr_inputs.diffuseColor / M_PI;
}

// Calculation of the lighting contribution from an optional Image Based Light source.
// Precomputed Environment Maps are required uniform inputs and are computed as outlined in [1].
// See our README.md on Environment Maps [3] for additional discussion.
fn get_ibl_contribution(pbr_inputs: PBRInfo, n: vec3<f32>, reflection: vec3<f32>) -> vec3<f32> {
	let lod = (pbr_inputs.perceptualRoughness * lighting_params.prefiltered_cube_mip_levels);
	// retrieve a scale and bias to F0. See [1], Figure 3
	let brdf = textureSample(t_brdf_lut, s_brdf_lut, vec2(pbr_inputs.NdotV, 1.0 - pbr_inputs.perceptualRoughness)).rgb;
	let diffuse_light = SRGBtoLINEAR(tonemap(textureSample(t_irradiance, s_irradiance, n))).rgb;
	let specular_light = SRGBtoLINEAR(tonemap(textureSampleLevel(t_prefiltered_env, s_prefiltered_env, reflection, lod))).rgb;

	let diffuse = diffuse_light * pbr_inputs.diffuseColor;
	let specular = specular_light * (pbr_inputs.specularColor * brdf.x + brdf.y);

	// For presentation, this allows us to disable IBL terms
	//diffuse *= lighting_params.scale_ibl_ambient;
	//specular *= lighting_params.scale_ibl_ambient;

	return diffuse + specular;
}

fn uncharted2_tonemap(color: vec3<f32>) -> vec3<f32> {
	let A = 0.15; let B = 0.50; let C = 0.10; let D = 0.20;
	let E = 0.02; let F = 0.30; let W = 11.2;
	return ((color*(A*color+C*B)+D*E)/(color*(A*color+B)+D*F))-E/F;
}

fn tonemap(color: vec4<f32>) -> vec4<f32> {
	var outcol = uncharted2_tonemap(color.rgb * lighting_params.exposure);
	outcol = outcol * (1.0f / uncharted2_tonemap(vec3(11.2f)));
	return vec4(pow(outcol, vec3(1.0f / lighting_params.gamma)), color.a);
}