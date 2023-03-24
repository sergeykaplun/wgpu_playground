use std::{io::{Cursor, BufReader}, borrow::Borrow};

use glm::mat4;
use gltf::Gltf;
use image::GenericImageView;
use wgpu::{util::{DeviceExt, BufferInitDescriptor}, BufferUsages, Device, Queue, BindGroup, BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, BindGroupEntry};

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
pub const MATERIAL_BGL: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor{
    label: Some("Texture bgl"),
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
    ],
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos:    [f32; 3],
    uv:     [f32; 2],
    normal: [f32; 3],
    color:  [f32; 3]
}
impl Vertex {
    fn default() -> Self {
        Self {
            pos:        [0.0; 3],
            normal:     [0.0; 3],
            uv:         [0.0; 2],
            color:      [0.0; 3]
        }
    }
}

struct Material {
    _base_color_factor: [f32; 4],
    base_color_texture_index: u32
}

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
    matrix:         glm::Mat4,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            parent: Default::default(),
            //children: Default::default(),
            mesh: Mesh { primitives: Default::default() },
            matrix: Default::default()
        }
    }
}

pub struct GLTFModel {
    nodes:              Vec<Node>,
    
    vertex_buffer:      wgpu::Buffer,
    index_buffer:       wgpu::Buffer,
    //indices_cnt:        u32,

    textures:           Vec<Option<BindGroup>>,
    materials:          Vec<Material>,
    nodes_matrices:     Vec<BindGroup>
}

impl GLTFModel {
    fn new(device: &Device, gltf: Gltf, materials: Vec<Material>, textures: Vec<Option<BindGroup>>, buffer_data: Vec<Vec<u8>>) -> GLTFModel {
        let mut index_buffer = Vec::new();
        let mut vertex_buffer = Vec::new();
        let mut nodes = Vec::<Node>::new();
        if let Some(scene) = gltf.scenes().nth(0) {
            scene.nodes().for_each(|node|{
                Self::load_node(&node, &gltf, None, &mut index_buffer, &mut vertex_buffer, &buffer_data, &mut nodes);
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
            textures,
            materials,
            nodes_matrices
        }
    }

    fn load_node(input_node: &gltf::Node, gltf: &Gltf, parent: Option<u32>, index_buffer: &mut Vec<u32>,  vertex_buffer: &mut Vec<Vertex>, buffer_data: &Vec<Vec<u8>>, nodes: &mut Vec<Node>) {
        let mut cur_node = Node::default();
		cur_node.parent = parent;

        let cur_node_index = nodes.len() as u32;
        nodes.push(cur_node);

        //FIXME
        nodes[cur_node_index as usize].matrix = mat4(
            input_node.transform().matrix()[0][0], input_node.transform().matrix()[1][0], input_node.transform().matrix()[2][0], input_node.transform().matrix()[3][0],
            input_node.transform().matrix()[0][1], input_node.transform().matrix()[1][1], input_node.transform().matrix()[2][1], input_node.transform().matrix()[3][1],
            input_node.transform().matrix()[0][2], input_node.transform().matrix()[1][2], input_node.transform().matrix()[2][2], input_node.transform().matrix()[3][2],
            input_node.transform().matrix()[0][3], input_node.transform().matrix()[1][3], input_node.transform().matrix()[2][3], input_node.transform().matrix()[3][3],
        );
        
        for child in input_node.children() {
            Self::load_node(&child, gltf, Some(cur_node_index), index_buffer, vertex_buffer, buffer_data, nodes);
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
                                vertex.uv = iter.into_f32().nth(i).unwrap().into();
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
                    let tex_index = model.materials[primitive.material_index as usize].base_color_texture_index as usize;
                    let bg = &model.textures[tex_index].as_ref().unwrap();
                    self.set_bind_group(3, bg, &[]);
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
    let textures = gltf.textures().map(|cur_tex| {
        let cur_image = match cur_tex.source().source() {
            gltf::image::Source::Uri { uri, .. } => {
                if !uri.contains("baseColor") {
                    return None
                }
                let new_uri = format!("{}{}", "models/FlightHelmet/glTF/", uri);
                let data = resource_manager.load_binary(&new_uri).unwrap();
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
        let cur_texture = device.create_texture(
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
                texture: &cur_texture,
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

        let cur_texture_view = cur_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cur_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Some(device.create_bind_group(&BindGroupDescriptor{
            label: Some(&format!("bg for {}", cur_tex.name().unwrap_or("noname"))),
            layout: &device.create_bind_group_layout(&MATERIAL_BGL),
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cur_texture_view),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&cur_sampler),
                },
            ],
        }))
    }).collect();
    let materials = gltf.materials().map(|cur_material| {
        Material{
            _base_color_factor: cur_material.pbr_metallic_roughness().base_color_factor(),
            base_color_texture_index: match cur_material.pbr_metallic_roughness().base_color_texture() {
                Some(tex) => tex.texture().index() as u32,
                None => 0,
            },
        }
    }).collect();
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                let new_uri = format!("{}{}", "models/FlightHelmet/glTF/", uri);
                let bin = resource_manager.load_binary(&new_uri).unwrap();
                buffer_data.push(bin);
            },
            _ => panic!("AAAAAAA")
        }
    }

    GLTFModel::new(device, gltf, materials, textures, buffer_data)
}

