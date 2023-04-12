use image::GenericImageView;
use wgpu::{BindGroup, Buffer, Device, Queue, util::{BufferInitDescriptor, DeviceExt}, BufferUsages, BindGroupLayout, TextureDescriptor, Origin3d, ImageCopyTexture, ImageDataLayout, Extent3d, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, ShaderModuleDescriptor, FragmentState, MultisampleState, RenderPass};

use crate::{app::ShaderType, geometry_primitives::{CUBE_DATA, CUBE_INDICES}, assets_helper::ResourceManager};

struct Renderer {
    pipeline: wgpu::RenderPipeline,
    skybox_texture_bg: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    cube_index_count: u32
}

pub struct Skybox {
    renderer : Renderer,
}

impl Skybox {
    pub fn new(device: &Device, queue: &Queue, resource_manager: &dyn ResourceManager,
               tex_format: TextureFormat, shader_type: ShaderType, camera_bgl: &BindGroupLayout) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: None,
            contents: bytemuck::cast_slice(CUBE_DATA),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: None,
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: BufferUsages::INDEX,
        });
        let skybox_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
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
        });
        let skybox_texture_bg = Self::create_skybox_texture(device, &queue, resource_manager, &skybox_bgl);
        let pipeline = Self::create_skybox_pipeline(device, camera_bgl, &skybox_bgl, tex_format, shader_type);
        
        Self{
            renderer: Renderer {
                          pipeline,
                          skybox_texture_bg,
                          vertex_buffer,
                          index_buffer,
                          cube_index_count: CUBE_INDICES.len() as u32
                      }
        }
    }

    fn create_skybox_pipeline(
        device: &wgpu::Device,
        camera_bgl: &BindGroupLayout,
        skybox_bgl: &BindGroupLayout,
        tex_format: TextureFormat,
        _: ShaderType
    ) -> wgpu::RenderPipeline {
        let buffer_layout = [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }
                ],
            }
        ];

        let color_states = [Some(ColorTargetState {
            format: tex_format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::REPLACE,
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let shader_module = device.create_shader_module(ShaderModuleDescriptor{
            label: Some("WGSL shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/skybox.wgsl").into()),
        });
        let vertex_state = wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &buffer_layout,
        };
        let fragment_state = FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &color_states
        };

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Skybox pipeline layout"),
                bind_group_layouts: &[skybox_bgl, camera_bgl],
                push_constant_ranges: &[],
            }
        );

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn create_skybox_texture(device: &Device, queue: &Queue, resource_manager: &dyn ResourceManager, skybox_bgl: &BindGroupLayout) -> BindGroup {
        let tex_face_names = [
            "textures/pond_skybox/posx.jpg", "textures/pond_skybox/negx.jpg",
            "textures/pond_skybox/posy.jpg", "textures/pond_skybox/negy.jpg",
            "textures/pond_skybox/posz.jpg", "textures/pond_skybox/negz.jpg",
        ];

        let face_size = image::load_from_memory(&resource_manager.load_binary(tex_face_names[0]).unwrap()).unwrap().dimensions();
        let skybox_size = wgpu::Extent3d {
            width: face_size.0,
            height: face_size.1,
            depth_or_array_layers: 6,
        };
        
        let skybox_texture = device.create_texture(&TextureDescriptor{
            label: Some("Skybox face"),
            size: skybox_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for i in 0..6u32 {
            let origin = Origin3d {x: 0, y: 0, z: i};
            let face_img = image::load_from_memory(&resource_manager.load_binary(tex_face_names[i as usize]).unwrap()).ok().unwrap();
            let face_rgba = face_img.to_rgba8();
            queue.write_texture(
                ImageCopyTexture {
                    texture: &skybox_texture,
                    mip_level: 0,
                    origin,
                    aspect: wgpu::TextureAspect::All,
                },
                &face_rgba,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(face_size.0 * 4),
                    rows_per_image: std::num::NonZeroU32::new(face_size.1),
                },
                Extent3d{
                    width: face_size.0,
                    height: face_size.1,
                    depth_or_array_layers: 1,
                }
            );
        }
        let tv = skybox_texture.create_view(&&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..wgpu::TextureViewDescriptor::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: skybox_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tv),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        })
    }
}

pub(crate) trait Drawable<'a> {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a BindGroup);
}

impl<'a, 'b> Drawable<'b> for RenderPass<'a> where 'b: 'a, {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a BindGroup) {
        self.set_pipeline(&skybox.renderer.pipeline);
        self.set_bind_group(0, &skybox.renderer.skybox_texture_bg, &[]);
        self.set_bind_group(1, &camera_bind_group, &[]);
        self.set_vertex_buffer(0, skybox.renderer.vertex_buffer.slice(..));
        self.set_index_buffer(skybox.renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.draw_indexed(0..skybox.renderer.cube_index_count, 0, 0..1);
    }
}