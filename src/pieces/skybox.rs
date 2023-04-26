use std::num::NonZeroU32;
use image::GenericImageView;
use wgpu::{BindGroup, Buffer, Device, Queue, util::{BufferInitDescriptor, DeviceExt}, BufferUsages, BindGroupLayout, TextureDescriptor, Origin3d, ImageCopyTexture, ImageDataLayout, Extent3d, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, ShaderModuleDescriptor, FragmentState, MultisampleState, RenderPass, DepthStencilState, StencilState, DepthBiasState, Texture};

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

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl Skybox {
    pub fn new(device: &Device, queue: &Queue, resource_manager: &dyn ResourceManager,
               tex_format: TextureFormat, shader_type: ShaderType, camera_bgl: &BindGroupLayout,
               generate_mips: bool) -> Self {
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
        //let pipeline_no_depth = Self::create_skybox_pipeline(device, camera_bgl, &skybox_bgl, TextureFormat::Rgba8UnormSrgb, false, shader_type);

        if generate_mips && num_mips > 0 {
            // let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            Self::generate_mipmaps(
                //&mut init_encoder,
                device,
                &queue,
                &vertex_buffer,
                &index_buffer,
                CUBE_INDICES.len() as u32,
                &skybox_texture,
                //&pipeline_no_depth,
                //&skybox_texture_bg,
                num_mips,
            );
            // queue.submit(Some(init_encoder.finish()));

            // Self::generate_mipmaps(
            //     device,
            //     &queue,
            //     &skybox_texture,
            //     num_mips,
            // );
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

    fn create_mip_generation_pipeline(
        device: &wgpu::Device,
        mm_bgl: &BindGroupLayout,
        skybox_bgl: &BindGroupLayout,
        tex_format: TextureFormat,
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
            label: Some("Mip generation shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/mip_generation.wgsl").into()),
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
                label: Some("Mip generation pipeline layout"),
                bind_group_layouts: &[skybox_bgl, mm_bgl],
                push_constant_ranges: &[],
            }
        );

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mip generation pipeline"),
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

        let tv = skybox_texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_mip_level: 0,
            mip_level_count: NonZeroU32::new(1),
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

    /*
    fn generate_mipmaps(
        //encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &Queue,
        //vertex_buffer: &Buffer,
        //index_buffer: &Buffer,
        //index_count: u32,
        texture: &wgpu::Texture,
        //pipeline: &wgpu::RenderPipeline,
        //bind_group: &BindGroup,
        mip_count: u32,
    ) {
        let input_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ],
        });
        let output_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2
                        },
                        count: None,
                    }
                ],
            label: Some("generate mip output bgl"),
        });
        let resolution_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Mip generation pipeline layout"),
            bind_group_layouts: &[&input_bgl, &output_bgl, &resolution_bgl],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Mip generation shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/wgsl/blit.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Mip generation pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "generate_mip",
        });
        let resolution_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Resolution buffer"),
            contents: bytemuck::bytes_of(&[0.0; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let resolution_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &resolution_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: resolution_buffer.as_entire_binding(),
            }],
            label: None,
        });
        
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None, });
        for face_index in 0..6 as u32{
            // let topmost_mip = texture.create_view(&wgpu::TextureViewDescriptor {
            //     label: None,
            //     dimension: Some(wgpu::TextureViewDimension::D2),
            //     base_mip_level: 0,
            //     mip_level_count: NonZeroU32::new(1),
            //     base_array_layer: face_index,
            //     array_layer_count: None,
            //     format: Some(TextureFormat::Rgba8Unorm),
            //     ..wgpu::TextureViewDescriptor::default()
            // });
            // let input_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            //     layout: &input_bgl,
            //     entries: &[
            //         wgpu::BindGroupEntry {
            //             binding: 0,
            //             resource: wgpu::BindingResource::TextureView(&topmost_mip),
            //         },
            //         wgpu::BindGroupEntry {
            //             binding: 1,
            //             resource: wgpu::BindingResource::Sampler(&sampler),
            //         },
            //     ],
            //     label: None,
            // });
            for target_mip in 1..mip_count as u32 {
                let topmost_mip = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: target_mip - 1,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: face_index,
                    array_layer_count: None,
                    format: Some(TextureFormat::Rgba8Unorm),
                    ..wgpu::TextureViewDescriptor::default()
                });
                let input_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &input_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&topmost_mip),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                    label: None,
                });
                let output_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &output_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(&wgpu::TextureViewDescriptor {
                                label: None,
                                dimension: Some(wgpu::TextureViewDimension::D2),
                                base_mip_level: target_mip as u32,
                                mip_level_count: NonZeroU32::new(1),
                                base_array_layer: face_index,
                                array_layer_count: None,
                                ..wgpu::TextureViewDescriptor::default()
                            })),
                        }
                    ],
                    label: None,
                });
                let cur_resolution = 1 << (mip_count - target_mip);
                queue.write_buffer(&resolution_buffer, 0, bytemuck::bytes_of(&[cur_resolution as f32; 4]));
                const WORKGROUP_SIZE: u32 = 8;
                let dispatch_count = (cur_resolution / WORKGROUP_SIZE).max(1);
                encoder.push_debug_group(format!("mip generation for face {} and level {}", face_index, target_mip).as_str());
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Shadow") });
                    cpass.set_pipeline(&pipeline);
                    cpass.set_bind_group(0, &input_bindgroup, &[]);
                    cpass.set_bind_group(1, &output_bindgroup, &[]);
                    cpass.set_bind_group(2, &resolution_bindgroup, &[]);
                    cpass.dispatch_workgroups(dispatch_count, dispatch_count, 1);
                    //cpass.dispatch_workgroups(self.renderer.work_group_count, self.renderer.work_group_count, 1);
                }
                encoder.pop_debug_group();
            }
        }
        queue.submit(iter::once(encoder.finish()));
    }
    */
    
    fn generate_mipmaps(
        device: &wgpu::Device,
        queue: &Queue,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
        index_count: u32,
        texture: &wgpu::Texture,
        //bind_group: &BindGroup,
        mip_count: u32,
    ) {
        let skybox_tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let matrix_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let mip_generation_pipeline = Self::create_mip_generation_pipeline(device, &matrix_bgl, &skybox_tex_bgl, TextureFormat::Rgba8UnormSrgb, ShaderType::WGSL);
        let model_matrices = [
            glm::rotate(&glm::Mat4::identity(), 90.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), -90.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), -90.0_f32.to_radians(), &glm::Vec3::new(1.0, 0.0, 0.0)),
            glm::rotate(&glm::Mat4::identity(), 90.0_f32.to_radians(), &glm::Vec3::new(1.0, 0.0, 0.0)),
            glm::Mat4::identity(),
            glm::rotate(&glm::Mat4::identity(), 180.0_f32.to_radians(), &glm::Vec3::new(0.0, 1.0, 0.0)),
        ];
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
                mip_level_count: NonZeroU32::new(1),
                ..wgpu::TextureViewDescriptor::default()
            });
            let mip_sample_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &skybox_tex_bgl,
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
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: face_index,
                    array_layer_count: None,
                });
                {
                    let mut uniform = Vec::<f32>::new();
                    uniform.extend(model_matrices[face_index as usize].iter());
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
}

pub(crate) trait DrawableSkybox<'a> {
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