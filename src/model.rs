use std::{io::{Cursor, BufReader}, borrow::Borrow};

use glm::mat4;
use gltf::{Gltf, material::AlphaMode, Semantic};
use image::GenericImageView;
use wgpu::{util::{DeviceExt, BufferInitDescriptor}, BufferUsages, Device, Queue, BindGroup, BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, BindGroupEntry, Texture, TextureViewDescriptor, TextureView, Sampler};

use crate::assets_helper::ResourceManager;

pub const NOD_MM_BGL:  BindGroupLayoutDescriptor = BindGroupLayoutDescriptor{
    label: Some("mm_bgl"),
    entries: &[BindGroupLayoutEntry{
        binding: 0,
        visibility: ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None
        },
        count: None,
    }],
};
// pub const MATERIAL_BGL: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor{
//     label: Some("Texture bgl"),
//     entries: &[
//         wgpu::BindGroupLayoutEntry {
//             binding: 0,
//             visibility: wgpu::ShaderStages::FRAGMENT,
//             ty: wgpu::BindingType::Texture {
//                 multisampled: false,
//                 view_dimension: wgpu::TextureViewDimension::D2,
//                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
//             },
//             count: None,
//         },
//         wgpu::BindGroupLayoutEntry {
//             binding: 1,
//             visibility: wgpu::ShaderStages::FRAGMENT,
//             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//             count: None,
//         },
//     ],
// };


// TODO make more elegant
pub const MATERIAL_BGL: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor{
    label: Some("Material bgl"),
    entries: &[
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 3,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 4,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 5,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 6,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 7,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 8,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 9,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 10,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None
        },
        count: None,
    },
    ]
};

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos:    [f32; 3],
    normal: [f32; 3],
    uv0:    [f32; 2],
    uv1:    [f32; 2],
    color:  [f32; 3]
}

#[derive(Default)]
struct Material {
    alpha_mode: AlphaMode,
    alpha_cutoff: f32,
    metallic_factor: f32,
    roughness_factor: f32,
    base_color_factor: [f32; 4],
    emissive_factor: [f32; 4],
    base_color_texture_index: Option<u32>,
    metallic_roughness_texture_index: Option<u32>,
    normal_texture_index: Option<u32>,
    occlusion_texture_index: Option<u32>,
    emissive_texture_index: Option<u32>,
    double_sided: bool,
    tex_coord_sets: TexCoordSets,
    
    bind_group: Option<BindGroup>,
    // extension: Extension,
    // pbr_workflows: PbrWorkflows,
    // descriptor_set: vk::DescriptorSet,
}

#[derive(Default)]
struct TexCoordSets {
    base_color: u8,
    metallic_roughness: u8,
    specular_glossiness: u8,
    normal: u8,
    occlusion: u8,
    emissive: u8,
}

// #[derive(Default)]
// struct Extension {
//     specular_glossiness_texture_index: Option<u32>,
//     diffuse_texture_index: Option<u32>,
//     diffuse_factor: [f32; 4],
//     specular_factor: [f32; 3],
// }

// #[derive(Default)]
// struct PbrWorkflows {
//     metallic_roughness: bool,
//     specular_glossiness: bool,
// }

// enum AlphaMode {
//     Opaque,
//     Mask,
//     Blend,
// }

// impl Default for AlphaMode {
//     fn default() -> Self {
//         Self::Opaque
//     }
// }

// enum AlphaMode{ ALPHAMODE_OPAQUE, ALPHAMODE_MASK, ALPHAMODE_BLEND }
// struct Material {
//     alpha_mode : AlphaMode,
//     alphaCutoff = 1.0f;
//     metallicFactor = 1.0f;
//     roughnessFactor = 1.0f;
//     _base_color_factor: [f32; 4],
//     base_color_texture_index: u32
// }

// impl Material {
//     fn default() -> Self {
//         Material {
//             alpha_mode: AlphaMode::ALPHAMODE_OPAQUE,
//             _base_color_factor: [0.0; 4],
//             base_color_texture_index: u32::MAX
//         }
//     }
// }

struct Primitive {
    first_index:     u32,
    index_count:     u32,
    material_index:  u32,
}

struct Mesh {
    primitives: Vec<Primitive>,
}

pub struct Node {
    parent:         Option<u32>,
    //children:       Vec<u32>,
    mesh:           Mesh,
    
    matrix:             glm::Mat4,
    // translation:        glm::Vec3,
    // scale:              glm::Vec3,
    // rotation:           glm::Quat,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            parent: None,
            mesh: Mesh { primitives: Vec::new() },
            matrix: glm::Mat4::identity(),
        }
    }
}

pub struct GLTFModel {
    nodes:              Vec<Node>,
    
    vertex_buffer:      wgpu::Buffer,
    index_buffer:       wgpu::Buffer,
    //indices_cnt:        u32,

    //textures:           Vec<Option<BindGroup>>,
    //textures:           Vec<Option<Texture>>,
    materials:          Vec<Material>,
    nodes_matrices:     Vec<BindGroup>
}

impl GLTFModel {
    fn new(device: &Device, gltf: Gltf, materials: Vec<Material>, /*textures: Vec<Option<Texture>>,*/ buffer_data: Vec<Vec<u8>>) -> GLTFModel {
        let mut index_buffer = Vec::new();
        let mut vertex_buffer = Vec::new();
        let mut nodes = Vec::<Node>::new();
        
        if let Some(scene) = gltf.scenes().nth(0) {
            scene.nodes().for_each(|node|{
                // let some = gltf.accessors().nth(node.mesh().unwrap().primitives().nth(0).unwrap().attributes().find(|semantic|{
                //     semantic.0 == Semantic::Positions
                // }));
                Self::load_node(&gltf, &node, &gltf, None, &mut index_buffer, &mut vertex_buffer, &buffer_data, &mut nodes);
            });
        }
        let nm_bgl = device.create_bind_group_layout(&NOD_MM_BGL);
        let nodes_matrices: Vec<BindGroup> = nodes.iter().map(|node| {
            let mut node_matrix = node.matrix;
            let mut current_parent_index = node.parent;
            while let Some(index) = current_parent_index {
                node_matrix = nodes[index as usize].matrix * node_matrix;
                current_parent_index = nodes[index as usize].parent;
            };
            let data: [[f32; 4]; 4] = node_matrix.into();
            let buffer = device.create_buffer_init(&BufferInitDescriptor{
                label: Some("model matrix buff"),
                contents: bytemuck::cast_slice(&data),
                usage: BufferUsages::UNIFORM,
            });
            device.create_bind_group(&BindGroupDescriptor{
                label: Some("Model matrix bg"),
                layout: &nm_bgl,
                entries: &[BindGroupEntry{
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            })
        }).collect();

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Gltf vertex data"),
            contents: bytemuck::cast_slice(&vertex_buffer),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Gltf vertex data"),
            contents: bytemuck::cast_slice(&index_buffer),
            usage: BufferUsages::INDEX,
        });

        Self {
            nodes,
            vertex_buffer,
            index_buffer,
            //indices_cnt,
            //textures,
            materials,
            nodes_matrices
        }
    }

    fn load_node(model: &Gltf, input_node: &gltf::Node, gltf: &Gltf, parent: Option<u32>, index_buffer: &mut Vec<u32>,  vertex_buffer: &mut Vec<Vertex>, buffer_data: &Vec<Vec<u8>>, nodes: &mut Vec<Node>) {
        let mut cur_node = Node::default();
		cur_node.parent = parent;

        let cur_node_index = nodes.len() as u32;
        nodes.push(cur_node);

        // FIXME flatten()
        nodes[cur_node_index as usize].matrix = mat4(
            input_node.transform().matrix()[0][0], input_node.transform().matrix()[1][0], input_node.transform().matrix()[2][0], input_node.transform().matrix()[3][0],
            input_node.transform().matrix()[0][1], input_node.transform().matrix()[1][1], input_node.transform().matrix()[2][1], input_node.transform().matrix()[3][1],
            input_node.transform().matrix()[0][2], input_node.transform().matrix()[1][2], input_node.transform().matrix()[2][2], input_node.transform().matrix()[3][2],
            input_node.transform().matrix()[0][3], input_node.transform().matrix()[1][3], input_node.transform().matrix()[2][3], input_node.transform().matrix()[3][3],
        );

        // cur_node.translation = glm::make_vec3(&input_node.transform().decomposed().0);
        // cur_node.rotation = glm::make_quat(&input_node.transform().decomposed().1);
        // cur_node.scale = glm::make_vec3(&input_node.transform().decomposed().2);

        for child in input_node.children() {
            Self::load_node(model, &child, gltf, Some(cur_node_index), index_buffer, vertex_buffer, buffer_data, nodes);
        }

        if let Some(mesh) = input_node.mesh() {
            for primitive in mesh.primitives() {
                let first_index: u32 = index_buffer.len() as u32;
                let vertex_start: u32 = vertex_buffer.len() as u32;
                let mut index_count = 0u32;
            
                // vertices
                {
                    let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
                    if let Some(iter) = reader.read_positions() {
                        for (i, pos) in iter.enumerate() {
                            let mut vertex = Vertex::default();
                            vertex.pos = pos.into();
                            
                            if let Some(iter) = reader.read_tex_coords(0) {
                                vertex.uv0 = iter.into_f32().nth(i).unwrap().into();
                            }
                            if let Some(iter) = reader.read_tex_coords(1) {
                                vertex.uv1 = iter.into_f32().nth(i).unwrap().into();
                            }
                            if let Some(mut iter) = reader.read_normals() {
                                vertex.normal = iter.nth(i).unwrap().into();
                            }
                            if let Some(iter) = reader.read_colors(0) {
                                vertex.color = iter.into_rgb_f32().nth(i).unwrap().into();
                            } else {
                                vertex.color = [1.0; 3];
                            }
                            vertex_buffer.push(vertex);
                        }
                    }
                    
                    if let Some(iter) = reader.read_indices() {
                        for index in iter.into_u32() {
                            index_buffer.push(index + vertex_start);
                            index_count += 1;
                        }
                    }
                }

                let primitive = Primitive{
                    first_index,
                    index_count,
                    material_index: primitive.material().index().unwrap() as u32,
                };
                nodes[cur_node_index as usize].mesh.primitives.push(primitive);
            }
        }
    }
}

pub(crate) trait Drawable<'a> {
    fn draw_model(&mut self, model: &'a GLTFModel, mode_mm_bg_index: u32);
    fn draw_node(&mut self, node: &Node, model: &'a GLTFModel);
}

impl<'a, 'b> Drawable<'b> for wgpu::RenderPass<'a> where 'b: 'a, {
    fn draw_model(&mut self, model: &'a GLTFModel, mode_mm_bg_index: u32) {
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        let nodes = &model.nodes;
        for (i, node) in nodes.iter().enumerate() {
            self.set_bind_group(mode_mm_bg_index, &model.nodes_matrices[i], &[]);
            self.draw_node(&node, &model);
        }
    }

    fn draw_node(&mut self, node: &Node, model: &'a GLTFModel) {
        if node.mesh.primitives.len() > 0 {
            let mesh = node.mesh.borrow();
            let primives: &Vec<Primitive> = &mesh.primitives;
            for primitive in primives {
                if primitive.index_count > 0 {
                    // TODO
                    //let tex_index = model.materials[primitive.material_index as usize].base_color_texture_index as usize;
                    //let bg = &model.textures[tex_index].as_ref().unwrap();
                    //self.set_bind_group(3, bg, &[]);
                    let bg = model.materials[primitive.material_index as usize].bind_group.as_ref().unwrap();
                    self.set_bind_group(1, bg, &[]);
                    self.draw_indexed(primitive.first_index..primitive.first_index + primitive.index_count, 0, 0..1);
                }
            }
        }
    }
}

pub async fn parse_gltf<T: ResourceManager>(file_name: &str, device: &wgpu::Device, queue: &Queue, resource_manager: &T) -> GLTFModel {
    let gltf_text = resource_manager.load_string(file_name).ok().unwrap();
    let gltf_cursor = Cursor::new(gltf_text);
    let gltf_reader = BufReader::new(gltf_cursor);
    let gltf = Gltf::from_reader(gltf_reader).ok().unwrap();
    //let textures: Vec<Option<Texture>> = gltf.textures().map(|cur_tex| {
    // TODO no need for opt
    let textures: Vec<Option<(TextureView, Sampler)>> = gltf.textures().map(|cur_tex| {
        let cur_image = match cur_tex.source().source() {
            gltf::image::Source::Uri { uri, .. } => {
                // if !uri.contains("baseColor") {
                //     return None
                // }
                // TODO
                //let new_uri = format!("{}{}", "models/FlightHelmet/glTF/", uri);
                //let data = resource_manager.load_binary(&new_uri).unwrap();
                
                let data = resource_manager.load_base64(uri).unwrap();

                image::load_from_memory(&data).unwrap()
            },
            _ => panic!("AAAAAAAAAAAAAA")
        };

        let cur_rgba = cur_image.to_rgba8();
        let (cur_width, cur_height) = cur_image.dimensions();

        let cur_size = wgpu::Extent3d {
            width: cur_width,
            height: cur_height,
            depth_or_array_layers: 1,
        };
        let wgpu_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: cur_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("font atlas texture"),
                view_formats: &[],
            }
        );
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &cur_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * cur_width),
                rows_per_image: std::num::NonZeroU32::new(cur_height),
            },
            cur_size,
        );
        //Some(wgpu_texture)
        let cur_texture_view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cur_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Some((cur_texture_view, cur_sampler))
        // Some(device.create_bind_group(&BindGroupDescriptor{
        //     label: Some(&format!("bg for {}", cur_tex.name().unwrap_or("noname"))),
        //     layout: &device.create_bind_group_layout(&MATERIAL_BGL),
        //     entries: &[
        //         BindGroupEntry{
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&cur_texture_view),
        //         },
        //         BindGroupEntry{
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&cur_sampler),
        //         },
        //     ],
        // })
        //)
    }).collect();
    
    let (empty_texture_view, default_sampler) = {
        let empty_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("1x1 tex"),
                view_formats: &[],
            }
        );
        
        let empty_texture_view = empty_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let empty_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        (empty_texture_view, empty_sampler)
    };

    let material_bgl = &device.create_bind_group_layout(&MATERIAL_BGL);
    let materials = gltf.materials().map(|cur_material| {
        let mut material = Material::default();
        material.double_sided = cur_material.double_sided();
        (material.base_color_texture_index,
         material.tex_coord_sets.base_color)  = match cur_material.pbr_metallic_roughness().base_color_texture() {
            Some(tex) => (Some(tex.texture().index() as u32), tex.tex_coord() as u8),
            None => (None, u8::MAX),
        };
        (material.metallic_roughness_texture_index,
         material.tex_coord_sets.metallic_roughness) = match cur_material.pbr_metallic_roughness().metallic_roughness_texture() {
            Some(tex) => (Some(tex.texture().index() as u32), tex.tex_coord() as u8),
            None => (None, u8::MAX),
        };
        material.roughness_factor = cur_material.pbr_metallic_roughness().roughness_factor();
        material.metallic_factor = cur_material.pbr_metallic_roughness().metallic_factor();
        material.base_color_factor = cur_material.pbr_metallic_roughness().base_color_factor();

        (material.normal_texture_index,
         material.tex_coord_sets.normal) = match cur_material.normal_texture() {
            Some(tex) => (Some(tex.texture().index() as u32), tex.tex_coord() as u8),
            None => (None, u8::MAX),
        };
        (material.emissive_texture_index,
            material.tex_coord_sets.emissive) = match cur_material.emissive_texture() {
               Some(tex) => (Some(tex.texture().index() as u32), tex.tex_coord() as u8),
               None => (None, u8::MAX),
           };

        (material.occlusion_texture_index,
            material.tex_coord_sets.occlusion) = match cur_material.occlusion_texture() {
               Some(tex) => (Some(tex.texture().index() as u32), tex.tex_coord() as u8),
               None => (None, u8::MAX),
           };
        
        material.alpha_mode = cur_material.alpha_mode();
        material.alpha_cutoff = match cur_material.alpha_cutoff() {
            Some(val) => val,
            None => 0.5,
        };
        material.emissive_factor = [cur_material.emissive_factor()[0], cur_material.emissive_factor()[1], cur_material.emissive_factor()[2], 1.0];

        let m = Mat {
            base_color_factor: material.base_color_factor,
            base_color_texture_set: match material.base_color_texture_index {
                Some(index) => index as i32,
                None => -1,
            },
            physical_descriptor_texture_set: match material.metallic_roughness_texture_index {
                Some(index) => index as i32,
                None => -1,
            },
            normal_texture_set: match material.normal_texture_index {
                Some(index) => index as i32,
                None => -1,
            },
            occlusion_texture_set: match material.occlusion_texture_index {
                Some(index) => index as i32,
                None => -1,
            },
            emissive_texture_set: match material.emissive_texture_index {
                Some(index) => index as i32,
                None => -1,
            },
            metallic_factor: material.metallic_factor,
            roughness_factor: material.roughness_factor,
            alpha_mask: 1.0,
            alpha_mask_cutoff: material.alpha_cutoff,
            alignment: [0.0f32; 3],
        };
        let buff = device.create_buffer_init(&BufferInitDescriptor{
            label: Some(format!("material {} buffer", cur_material.name().unwrap_or("unnamed")).as_str()),
            contents: bytemuck::cast_slice(&[m]),
            usage: BufferUsages::UNIFORM,
        });

        material.bind_group = Some(device.create_bind_group(&BindGroupDescriptor{
            label: Some(format!("Material {} BG", cur_material.name().unwrap_or("unnamed")).as_str()),
            layout: &material_bgl,
            entries: &[
                // base color
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        match material.base_color_texture_index {
                            Some(ind) => &textures[ind as usize].as_ref().unwrap().0,
                            None => &empty_texture_view,
                        }
                        //&textures[material.base_color_texture_index.unwrap() as usize].as_ref().unwrap().0
                    )
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        match material.base_color_texture_index {
                            Some(ind) => &textures[ind as usize].as_ref().unwrap().1,
                            None => &default_sampler,
                        }
                        //&textures[material.base_color_texture_index.unwrap() as usize].as_ref().unwrap().1
                    )
                },

                // metallic roughness
                BindGroupEntry{
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[material.metallic_roughness_texture_index.unwrap() as usize].as_ref().unwrap().0
                    )
                },
                BindGroupEntry{
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(
                        &textures[material.metallic_roughness_texture_index.unwrap() as usize].as_ref().unwrap().1
                    )
                },

                // normal 
                BindGroupEntry{
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[material.normal_texture_index.unwrap() as usize].as_ref().unwrap().0
                    )
                },
                BindGroupEntry{
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(
                        &textures[material.normal_texture_index.unwrap() as usize].as_ref().unwrap().1
                    )
                },

                // occlusion 
                BindGroupEntry{
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[material.occlusion_texture_index.unwrap() as usize].as_ref().unwrap().0
                    )
                },
                BindGroupEntry{
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(
                        &textures[material.occlusion_texture_index.unwrap() as usize].as_ref().unwrap().1
                    )
                },

                // emissive 
                BindGroupEntry{
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(
                        match material.emissive_texture_index {
                            Some(ind) => &textures[ind as usize].as_ref().unwrap().0,
                            None => &empty_texture_view,
                        }
                        //&textures[material.emissive_texture_index.unwrap() as usize].as_ref().unwrap().0
                    )
                },
                BindGroupEntry{
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(
                        match material.emissive_texture_index {
                            Some(ind) => &textures[ind as usize].as_ref().unwrap().1,
                            None => &default_sampler,
                        }
                        //&textures[material.emissive_texture_index.unwrap() as usize].as_ref().unwrap().1
                    )
                },
                BindGroupEntry{
                    binding: 10,
                    resource: buff.as_entire_binding()
                },
            ],
        }));

        material
    }).collect();
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                //let new_uri = format!("{}{}", "models/FlightHelmet/glTF/", uri);
                //let bin = resource_manager.load_binary(&new_uri).unwrap();
                let bin = resource_manager.load_base64(uri).unwrap();
                buffer_data.push(bin);
            },
            _ => panic!("AAAAAAA")
        }
    }

    GLTFModel::new(device, gltf, materials, /*textures, */buffer_data)
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Mat {
	base_color_factor:                      [f32; 4],
    base_color_texture_set:                 i32,
	physical_descriptor_texture_set:        i32,
	normal_texture_set:                     i32,
	occlusion_texture_set:                  i32,
	emissive_texture_set:                   i32,
	metallic_factor:                        f32,
	roughness_factor:                       f32,
	alpha_mask:                             f32,
	alpha_mask_cutoff:                      f32,
    alignment:                              [f32; 3]
}