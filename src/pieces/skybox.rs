use std::num::NonZeroU32;
use image::{GenericImageView, ImageFormat};
use wgpu::{BindGroup, Buffer, Device, Queue, util::{BufferInitDescriptor, DeviceExt}, BufferUsages, BindGroupLayout, TextureDescriptor, Origin3d, ImageCopyTexture, ImageDataLayout, Extent3d, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, ShaderModuleDescriptor, FragmentState, MultisampleState, RenderPass, DepthStencilState, StencilState, DepthBiasState, Texture, TextureDimension, TextureUsages, TextureViewDescriptor, TextureViewDimension, TextureAspect, SamplerDescriptor, AddressMode, FilterMode, BindGroupDescriptor, TextureView, Sampler};
use ktx::{Ktx, include_ktx, KtxInfo};

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
    pub(crate) irradiance_tv: TextureView,
    pub(crate) irradiance_sampler: Sampler,
    pub(crate) prefiltered_envmap_tv: TextureView,
    pub(crate) prefiltered_envmap_sampler: Sampler,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl Skybox {
    pub fn new(device: &Device, queue: &Queue, resource_manager: &dyn ResourceManager,
               tex_format: TextureFormat, shader_type: ShaderType, camera_bgl: &BindGroupLayout,
               generate_mips: bool) -> Self {
        Self::default_ktx(device, queue, tex_format, shader_type, camera_bgl)
        /*
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
        let (skybox_texture, skybox_texture_bg, num_mips) = Self::create_skybox_texture(device, &queue, resource_manager, &skybox_bgl, generate_mips);
        let pipeline = Self::create_skybox_pipeline(device, camera_bgl, &skybox_bgl, tex_format, true, shader_type);

        if generate_mips && num_mips > 0 {
            Self::generate_mipmaps(
                device,
                &queue,
                &vertex_buffer,
                &index_buffer,
                CUBE_INDICES.len() as u32,
                &skybox_texture,
                num_mips,
            );
        }

        Self{
            renderer: Renderer {
                          pipeline,
                          skybox_texture_bg,
                          vertex_buffer,
                          index_buffer,
                          cube_index_count: CUBE_INDICES.len() as u32
                      }
        }
        */
    }

    pub fn default_ktx(device: &Device, queue: &Queue, tex_format: TextureFormat,
                       shader_type: ShaderType, camera_bgl: &BindGroupLayout) -> Self {
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
        //let pipeline = Self::create_skybox_pipeline(device, camera_bgl, &skybox_bgl, tex_format, true, shader_type);
        let pipeline = Self::create_skybox_pipeline(device, camera_bgl, &skybox_bgl, tex_format, false, shader_type);
        //let ktx_image: Ktx<_> = include_ktx!("../../assets/textures/papermill.ktx");
        //let ktx_image: Ktx<_> = include_ktx!("../../assets/textures/stars.ktx");
        let ktx_image: Ktx<_> = include_ktx!("../../assets/empty.ktx");
        let mip_count = ktx_image.textures().count() as u32;
        let format = TextureFormat::Rgba16Float;
        let texture_size = Extent3d {
            width: ktx_image.pixel_width(),
            height: ktx_image.pixel_height(),
            depth_or_array_layers: 6,
        };
        let skybox_texture = device.create_texture(&TextureDescriptor {
            label: Some("Skybox Texture"),
            size: texture_size,
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for cur_mip in 0..mip_count {
            let cur_mip_data: &[u8] = ktx_image.textures().nth(cur_mip as usize).unwrap();
            let cur_mip_size = texture_size.width >> cur_mip;
            let face_span = cur_mip_data.len() / 6;
            for face_index in 0..6 {
                let start = face_span * face_index;
                let end = start + face_span;
                let face_data = &cur_mip_data[start..end];
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &skybox_texture,
                        mip_level: cur_mip,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: face_index as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    face_data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some((8 * cur_mip_size).into()),
                        rows_per_image: Some(cur_mip_size.into()),
                    },
                    Extent3d {
                        width: cur_mip_size,
                        height: cur_mip_size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let tv = skybox_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..wgpu::TextureViewDescriptor::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let skybox_texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &skybox_bgl,
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
        });

        let (irradiance_tv, irradiance_sampler) = Self::generate_irradiance(device, queue, &skybox_texture_bg, &vertex_buffer, &index_buffer, CUBE_INDICES.len() as u32);
        let (prefiltered_envmap_tv, prefiltered_envmap_sampler) = Self::generate_prefiltered_env_map(device, queue, &skybox_texture_bg, &vertex_buffer, &index_buffer, CUBE_INDICES.len() as u32);

        Self{
            renderer: Renderer {
                pipeline,
                skybox_texture_bg,
                //skybox_texture_bg: irradiance_bg,
                //skybox_texture_bg: prefiltered_envmap,
                vertex_buffer,
                index_buffer,
                cube_index_count: CUBE_INDICES.len() as u32
            },
            irradiance_tv,
            irradiance_sampler,
            prefiltered_envmap_tv,
            prefiltered_envmap_sampler,
        }
    }

    fn create_skybox_pipeline(
        device: &wgpu::Device,
        camera_bgl: &BindGroupLayout,
        skybox_bgl: &BindGroupLayout,
        tex_format: TextureFormat,
        use_depth: bool,
        _: ShaderType,
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
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                //front_face: wgpu::FrontFace::Cw,
                //cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: if use_depth {
                Some(DepthStencilState{
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                })
            } else {
                None
            },
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn create_cube_pipeline(device: &wgpu::Device, shader_module: &wgpu::ShaderModule, additional_bgl: Option<&[&BindGroupLayout]>, tex_format: TextureFormat) -> wgpu::RenderPipeline {
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

        let skybox_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
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
        let rotmat_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let mut bind_group_layouts = vec![&skybox_bgl, &rotmat_bgl];
        if let Some(additional_bgl) = additional_bgl {
            bind_group_layouts.extend_from_slice(additional_bgl);
        }
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Cube pipeline layout"),
                bind_group_layouts: &bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            }
        );

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn create_skybox_texture(device: &Device, queue: &Queue, resource_manager: &dyn ResourceManager,
                             skybox_bgl: &BindGroupLayout, generate_mips: bool) -> (Texture, BindGroup, u32) {
        let tex_face_names = [
            "textures/teide_skybox/posx.jpg", "textures/teide_skybox/negx.jpg",
            "textures/teide_skybox/posy.jpg", "textures/teide_skybox/negy.jpg",
            "textures/teide_skybox/posz.jpg", "textures/teide_skybox/negz.jpg",
        ];

        let face_size = image::load_from_memory(&resource_manager.load_binary(tex_face_names[0]).unwrap()).unwrap().dimensions();
        let skybox_size = wgpu::Extent3d {
            width: face_size.0,
            height: face_size.1,
            depth_or_array_layers: 6,
        };
        
        let num_mips = if generate_mips {
                                (f32::log2(face_size.0.min(face_size.1) as f32).floor() + 1.0) as u32
                            } else {
                                0
                            };

        let skybox_texture = device.create_texture(&TextureDescriptor{
            label: Some("Skybox face"),
            size: skybox_size,
            mip_level_count: num_mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            //format: wgpu::TextureFormat::Rgba8Unorm,
            //format: wgpu::TextureFormat::Rgba32Float,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            //format: wgpu::TextureFormat::Bgra8UnormSrgb,
            // TODO 
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
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
                    bytes_per_row: Some((face_size.0 * 4).into()),
                    rows_per_image: Some(face_size.1.into()),
                },
                Extent3d{
                    width: face_size.0,
                    height: face_size.1,
                    depth_or_array_layers: 1,
                }
            );
        }

        let tv = skybox_texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_mip_level: 0,
            mip_level_count: Some(1u32.into()),
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

        let skybox_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });
        (skybox_texture, skybox_bindgroup, num_mips)
    }

    fn generate_mipmaps(
        device: &wgpu::Device,
        queue: &Queue,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
        index_count: u32,
        texture: &wgpu::Texture,
        mip_count: u32,
    ) {
        let gen_mip_shader_module = device.create_shader_module(ShaderModuleDescriptor{
            label: Some("Mip generation shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/mip_generation.wgsl").into()),
        });
        let mip_generation_pipeline = Self::create_cube_pipeline(device, &gen_mip_shader_module, None, TextureFormat::Rgba8UnormSrgb);
        let arr = [0.0f32; 16];
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mip model Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mip_generation_pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some("model_bind_group"),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        
        const FACE_COUNT: u32 = 6;
        for target_mip in 1..mip_count as u32 {
            let sample_tv = texture.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                dimension: Some(wgpu::TextureViewDimension::Cube),
                base_mip_level: target_mip - 1,
                mip_level_count: Some(1u32.into()),
                ..TextureViewDescriptor::default()
            });
            let mip_sample_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &mip_generation_pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&sample_tv),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });
            for face_index in 0..FACE_COUNT {
                let tv = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(format!("tv_mip_{}_face_{}", target_mip, face_index).as_str()),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: target_mip,
                    mip_level_count: Some(1u32.into()),
                    base_array_layer: face_index,
                    array_layer_count: None,
                });
                {
                    let mut uniform = Vec::<f32>::new();
                    uniform.extend(Self::get_cube_rotmats()[face_index as usize].iter());
                    queue.write_buffer(&model_buffer, 0, bytemuck::cast_slice(&uniform));
                }
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.push_debug_group(format!("Skybox mip #{} pass for face {}", target_mip, face_index).as_str());
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &tv,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                    
                    rpass.set_pipeline(&mip_generation_pipeline);
                    //rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_bind_group(0, &mip_sample_bg, &[]);
                    rpass.set_bind_group(1, &model_bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.draw_indexed(0..index_count, 0, 0..1);
                }
                encoder.pop_debug_group();
                queue.submit(Some(encoder.finish()));
            }
        }
    }

    fn generate_irradiance(device: &Device, queue: &Queue, skybox_bg: &BindGroup,
                           vertex_buffer: &Buffer, index_buffer: &Buffer, index_count: u32,) -> (TextureView, Sampler) {
        let dim = 64u32;
        let format = wgpu::TextureFormat::Rgba16Float;
        let num_mips = 1 + (dim as f32).log2().floor() as u32;
        let texture = device.create_texture(&TextureDescriptor{
            label: Some("Irradiance Texture"),
            size: Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: 6,
            },
            mip_level_count: num_mips,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[]
        });

        let gen_irradiance_shader_module = device.create_shader_module(ShaderModuleDescriptor{
            label: Some("Irradiance generation shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/irradiance_gen.wgsl").into()),
        });
        let irradiance_generation_pipeline = Self::create_cube_pipeline(device, &gen_irradiance_shader_module, None, format);
        let arr = [0.0f32; 16];
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Irradiance model Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &irradiance_generation_pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some("irradiance_bind_group"),
        });

        const FACE_COUNT: u32 = 6;
        for target_mip in 0..num_mips as u32 {
            for face_index in 0..FACE_COUNT {
                let tv = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(format!("tv_mip_{}_face_{}", target_mip, face_index).as_str()),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: target_mip,
                    mip_level_count: Some(1u32.into()),
                    base_array_layer: face_index,
                    array_layer_count: None,
                });
                {
                    let mut uniform = Vec::<f32>::new();
                    uniform.extend(Self::get_cube_rotmats()[face_index as usize].iter());
                    queue.write_buffer(&model_buffer, 0, bytemuck::cast_slice(&uniform));
                }
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.push_debug_group(format!("Irradiance mip #{} pass for face {}", target_mip, face_index).as_str());
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &tv,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(&irradiance_generation_pipeline);
                    //rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_bind_group(0, &skybox_bg, &[]);
                    rpass.set_bind_group(1, &model_bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.draw_indexed(0..index_count, 0, 0..1);
                }
                encoder.pop_debug_group();
                queue.submit(Some(encoder.finish()));
            }
        }

        let texture_view = texture.create_view(&TextureViewDescriptor{
            label: Some("Irradiance Texture View"),
            format: Some(format),
            dimension: Some(TextureViewDimension::Cube),
            aspect: TextureAspect::All,
            mip_level_count: Some(num_mips.into()),
            array_layer_count: Some(6u32.into()),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor{
            label: Some("Irradiance Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: num_mips as f32,
            ..Default::default()
        });

        (texture_view, sampler)
        // device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &irradiance_generation_pipeline.get_bind_group_layout(0),
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&texture_view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&sampler),
        //         },
        //     ],
        //     label: None,
        // })
    }

    fn generate_prefiltered_env_map(device: &Device, queue: &Queue, skybox_bg: &BindGroup,
                           vertex_buffer: &Buffer, index_buffer: &Buffer, index_count: u32,) -> (TextureView, Sampler) {
        let dim = 512u32;
        let format = wgpu::TextureFormat::Rgba16Float;
        let num_mips = 1 + (dim as f32).log2().floor() as u32;
        let texture = device.create_texture(&TextureDescriptor{
            label: Some("Irradiance Texture"),
            size: Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: 6,
            },
            mip_level_count: num_mips,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[]
        });

        let gen_env_shader_module = device.create_shader_module(ShaderModuleDescriptor{
            label: Some("Prefiltered envmap generation shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/prefiltered_env_gen.wgsl").into()),
        });
        let roughness_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let envmap_generation_pipeline = Self::create_cube_pipeline(device, &gen_env_shader_module, Some(&vec![&roughness_bgl]), format);
        let arr = [0.0f32; 16];
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Envmap model Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &envmap_generation_pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some("envmap_bind_group"),
        });
        let arr = [0.0f32; 4];
        let roughness_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Envmap roughness Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let roughtness_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &envmap_generation_pipeline.get_bind_group_layout(2),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: roughness_buffer.as_entire_binding(),
            }],
            label: Some("envmap_roughness_bind_group"),
        });

        const FACE_COUNT: u32 = 6;
        for target_mip in 0..num_mips as u32 {
            {
                let uniform = [(target_mip as f32) / ((num_mips - 1) as f32); 4];
                queue.write_buffer(&roughness_buffer, 0, bytemuck::cast_slice(&uniform));
            }
            for face_index in 0..FACE_COUNT {
                let tv = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(format!("tv_mip_{}_face_{}", target_mip, face_index).as_str()),
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: target_mip,
                    mip_level_count: Some(1u32.into()),
                    base_array_layer: face_index,
                    array_layer_count: None,
                });
                {
                    let mut uniform = Vec::<f32>::new();
                    uniform.extend(Self::get_cube_rotmats()[face_index as usize].iter());
                    queue.write_buffer(&model_buffer, 0, bytemuck::cast_slice(&uniform));
                }
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                encoder.push_debug_group(format!("Envmap mip #{} pass for face {}", target_mip, face_index).as_str());
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &tv,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(&envmap_generation_pipeline);
                    //rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_bind_group(0, &skybox_bg, &[]);
                    rpass.set_bind_group(1, &model_bind_group, &[]);
                    rpass.set_bind_group(2, &roughtness_bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.draw_indexed(0..index_count, 0, 0..1);
                }
                encoder.pop_debug_group();
                queue.submit(Some(encoder.finish()));
            }
        }

        let texture_view = texture.create_view(&TextureViewDescriptor{
            label: Some("Envmap Texture View"),
            format: Some(format),
            dimension: Some(TextureViewDimension::Cube),
            aspect: TextureAspect::All,
            mip_level_count: Some(num_mips.into()),
            array_layer_count: Some(6u32.into()),
            ..Default::default()
        });
        let sampler = device.create_sampler(&SamplerDescriptor{
            label: Some("Envmap Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: num_mips as f32,
            ..Default::default()
        });

        (texture_view, sampler)
        // device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &envmap_generation_pipeline.get_bind_group_layout(0),
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&texture_view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&sampler),
        //         },
        //     ],
        //     label: None,
        // })
    }

    //fn generate_brdf_lut(device: &Device, queue: &Queue)

    fn get_cube_rotmats() -> [glm::Mat4; 6] {
        [
            glm::rotate(&glm::Mat4::identity(), 90.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), -90.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), -90.0_f32.to_radians(), &glm::Vec3::new(1.0, 0.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), 90.0_f32.to_radians(), &glm::Vec3::new(1.0, 0.0, 0.0)),
            glm::Mat4::identity(),
            glm::rotate(&glm::Mat4::identity(), 180.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
        ]
    }
}

pub trait DrawableSkybox<'a> {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a BindGroup);
}

impl<'a, 'b> DrawableSkybox<'b> for RenderPass<'a> where 'b: 'a, {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a BindGroup) {
        self.set_pipeline(&skybox.renderer.pipeline);
        self.set_bind_group(0, &skybox.renderer.skybox_texture_bg, &[]);
        self.set_bind_group(1, &camera_bind_group, &[]);
        self.set_vertex_buffer(0, skybox.renderer.vertex_buffer.slice(..));
        self.set_index_buffer(skybox.renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.draw_indexed(0..skybox.renderer.cube_index_count, 0, 0..1);
    }
}