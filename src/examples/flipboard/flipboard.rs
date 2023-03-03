use std::{iter, mem};

use wgpu::{Queue, RenderPipeline, ColorTargetState, TextureFormat, ShaderModule, VertexState, FragmentState, Device, ShaderModuleDescriptor, PipelineLayoutDescriptor, PrimitiveState, MultisampleState, TextureView, ComputePipelineDescriptor, BindGroupEntry, BindGroupDescriptor, ComputePipeline, BindGroup, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, util::{BufferInitDescriptor, DeviceExt}, BufferUsages, Buffer, VertexBufferLayout, VertexAttribute, BufferDescriptor, BindGroupLayout, RenderPassDepthStencilAttachment, Operations, DepthStencilState, StencilState, DepthBiasState, Color, RenderPipelineDescriptor, Sampler, BindingType};
use winit::event::WindowEvent;

use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}, assets_helper::{Mesh, self}};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SHADOW_TEX_SIZE: u32 = 1024;
const GAME_WORKGROUP_SIZE: u32 = 16;

struct Renderer {
    queue: Queue,
    depth_tex_view:                     TextureView,
    shadow_tex_view:                    TextureView,
    shadow_sampler:                     Sampler,

    constants_buffer:                   Buffer,
    flap_pad_instance_buffer:           Buffer,
    
    //globals_bindgroup:                  BindGroup,
    //light_bindgroup:                    BindGroup,
    global_data_bindgroup:              BindGroup,
    gamedata_write_bindgroup:           BindGroup,
    gamedata_read_bindgroup:            BindGroup,
    shadow_tex_bg:                      BindGroup,
    
    flaps_pipeline:                     RenderPipeline,
    game_compute_pipeline:              ComputePipeline,
    shadow_pipeline:                    RenderPipeline,
    
    game_compute_workgroups_count:      [u32; 2],
    flap_pad_instances_cnt:             u32,

    meshes:                             Vec<Mesh>,
}

pub(crate) struct FlipboardExample {
    renderer: Renderer,
    globals: Globals,
    camera: ArcballCamera,
}

impl App for FlipboardExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: crate::app::ShaderType
    ) -> Self {
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 90., 0.01, 100., 7., 1.);
        let camera_bindgroup_data = camera.get_bind_group(1);
        let global_data_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("Globals bgl"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT | ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
                camera_bindgroup_data.0,
                BindGroupLayoutEntry{
                    binding: 2,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                }
            ],
        });
        let game_buffer_write_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    }
                ],
            label: Some("game_output_bind_group_layout"),
        });
        let game_buffer_read_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    }
                ],
            label: Some("game_output_bind_group_layout"),
        });
        
        //let (light_bindgroup_layout, light_bindgroup) = Self::get_light_data(device);
        let (flap_pad_instance_buffer, gamedata_buffer, constants_buffer, light_data_buffer) = Self::create_buffers(device);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        let (shadow_tex_view, shadow_sampler, shadow_tex_bgl, shadow_tex_bg) = Self::create_shadow_texture(device);

        let game_compute_pipeline = Self::create_compute_pipeline(device, &global_data_bgl, &game_buffer_write_bgl, shader_type);
        let flaps_pipeline = Self::create_render_pipeline(device, sc.format, &global_data_bgl, &game_buffer_read_bgl, &shadow_tex_bgl, shader_type);
        let shadow_pipeline = Self::create_shadow_pipeline(device, &global_data_bgl, &game_buffer_read_bgl, shader_type);
        
        let global_data_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Global bg"),
            layout: &global_data_bgl,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: constants_buffer.as_entire_binding(),
                },
                camera_bindgroup_data.1,
                BindGroupEntry{
                    binding: 2,
                    resource: light_data_buffer.as_entire_binding(),
                }
            ],
        });
        let gamedata_write_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Gamedata bg"),
            layout: &game_buffer_write_bgl,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: gamedata_buffer.as_entire_binding(),
            }],
        });
        let gamedata_read_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Gamedata bg"),
            layout: &game_buffer_read_bgl,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: gamedata_buffer.as_entire_binding(),
            }],
        });
        
        let meshes = pollster::block_on(
            assets_helper::load_model(
                "111.obj",
                &device,
            )
        ).expect("Error while loading model");
        
        let renderer = Renderer {
            queue,
            depth_tex_view,
            shadow_tex_view,
            shadow_sampler,
            constants_buffer,
            flap_pad_instance_buffer,
            //globals_bindgroup,
            global_data_bindgroup,
            gamedata_write_bindgroup,
            gamedata_read_bindgroup,
            //light_bindgroup,
            shadow_tex_bg,
            flaps_pipeline,
            game_compute_pipeline,
            shadow_pipeline,
            game_compute_workgroups_count: FlapPad::RESOLUTION.map(|v| (v/GAME_WORKGROUP_SIZE).max(1)),
            flap_pad_instances_cnt: FlapPad::RESOLUTION[0] * FlapPad::RESOLUTION[1] * 3,
            meshes,
        };
        Self {
            renderer,
            globals: Globals {
                input_res: FlapPad::RESOLUTION.map(|v| v as f32),
                time: 0.0,
                time_delta: 0.0,
            },
            camera,
        }
    }

    fn render(&mut self, surface: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError> {
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        
        encoder.push_debug_group("game output pass");
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Game output") });
            cpass.set_pipeline(&self.renderer.game_compute_pipeline);
            cpass.set_bind_group(0, &self.renderer.global_data_bindgroup, &[]);
            cpass.set_bind_group(1, &self.renderer.gamedata_write_bindgroup, &[]);
            //cpass.dispatch_workgroups(self.renderer.game_compute_workgroups_count, self.renderer.game_compute_workgroups_count, 1);
            cpass.dispatch_workgroups(self.renderer.game_compute_workgroups_count[0], self.renderer.game_compute_workgroups_count[1], 1);
        }
        encoder.pop_debug_group();
        
        encoder.insert_debug_marker("shadow pass");
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.shadow_tex_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            pass.set_pipeline(&self.renderer.shadow_pipeline);
            //pass.set_bind_group(0, &self.renderer.light_bindgroup, &[]);
            //pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            pass.set_bind_group(0, &self.renderer.global_data_bindgroup, &[]);
            pass.set_bind_group(1, &self.renderer.gamedata_read_bindgroup, &[]);

            for mesh in &self.renderer.meshes {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.renderer.flap_pad_instance_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.num_elements, 0, 0..self.renderer.flap_pad_instances_cnt);
            }
        }
        encoder.pop_debug_group();

        {
            self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.globals]));

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.renderer.depth_tex_view,
                    depth_ops: Some(Operations{
                        load: wgpu::LoadOp::Clear(1.0),
                        store: false,
                    }),
                    stencil_ops: None,
                })
            });
        
            render_pass.set_pipeline(&self.renderer.flaps_pipeline);
            render_pass.set_bind_group(0, &self.renderer.global_data_bindgroup, &[]);
            render_pass.set_bind_group(1, &self.renderer.gamedata_read_bindgroup, &[]);
            render_pass.set_bind_group(2, &self.renderer.shadow_tex_bg, &[]);
            // render_pass.set_bind_group(2, &self.camera.camera_bind_group, &[]);
            // render_pass.set_bind_group(3, &self.renderer.light_bindgroup, &[]);
            render_pass.set_vertex_buffer(1, self.renderer.flap_pad_instance_buffer.slice(..));
            
            for mesh in &self.renderer.meshes {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_elements, 0, 0..self.renderer.flap_pad_instances_cnt);
                
                //render_pass.set_vertex_buffer(0, self.renderer.flap_pad_vb.slice(..));
                //render_pass.set_index_buffer(self.renderer.flap_pad_ib.slice(..), wgpu::IndexFormat::Uint16);
                //render_pass.draw_indexed(0..self.renderer.flap_pad_index_cnt, 0, 0..self.renderer.flap_pad_instances_cnt);
            }
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();
        
        self.globals.time_delta = 0.0;
        Ok(())
    }
    
    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
        self.globals.time += delta * 0.1;
        self.globals.time_delta += delta * 0.1;
    }

    fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }
}

impl FlipboardExample {
    fn create_render_pipeline(device: &Device, tex_format: TextureFormat, global_data_bgl: &BindGroupLayout, gamedata_bgl: &BindGroupLayout, shadow_tex_bgl: &BindGroupLayout, shader_type: ShaderType) -> RenderPipeline {
        let vertex_buffer_layout = [
            VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x2,
                        offset: mem::size_of::<[f32; 3]>() as u64,
                        shader_location: 1,
                    },
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x3,
                        offset: mem::size_of::<[f32; 5]>() as u64,
                        shader_location: 2,
                    }
                ],
            },
            VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 3,
                    },
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x2,
                        offset: mem::size_of::<[f32; 2]>() as u64,
                        shader_location: 4,
                    }
                ],
            },
        ];
        let color_states = [Some(ColorTargetState {
            format: tex_format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::REPLACE,
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let mut spirv_modules : Vec<ShaderModule> = vec![];

        let vertex_state: VertexState;
        let fragment_state: FragmentState;
        match shader_type {
            ShaderType::WGSL => {
                spirv_modules.push(device.create_shader_module(ShaderModuleDescriptor{
                    label: Some("WGSL shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/output.wgsl").into()),
                }));
                vertex_state = wgpu::VertexState {
                    module: &spirv_modules[0],
                    entry_point: "vs_main",
                    buffers: &vertex_buffer_layout,
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[0],
                    entry_point: "fs_main",
                    targets: &color_states
                }
            },
            _ => panic!("")
        }
        
        // let camera_bind_group_layout = device.create_bind_group_layout(
        //     &wgpu::BindGroupLayoutDescriptor {
        //         entries: &[wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::VERTEX,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         }],
        //     label: Some("camera_bind_group_layout"),
        // });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Output pipeline"),
            bind_group_layouts: &[global_data_bgl, &gamedata_bgl, shadow_tex_bgl],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Output pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: /*Some(Face::Back)*/None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false
            },
            depth_stencil: Some(DepthStencilState{
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn create_compute_pipeline(device: &Device, globals_bgl: &BindGroupLayout, gamedata_bgl: &BindGroupLayout, shader_type: ShaderType) -> ComputePipeline {
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Game output pipeline descriptor"),
            bind_group_layouts: &[globals_bgl, gamedata_bgl],
            push_constant_ranges: &[],
        });
        
        let shader_module = match shader_type {
            ShaderType::WGSL => device.create_shader_module(ShaderModuleDescriptor{
                                    label: Some("WGSL shader"),
                                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/game.wgsl").into()),
                                }),
            // ShaderType::SPIRV => unsafe {
            //                         device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/shadow.cs.spv"))
            //                     },
            _ => panic!("PANIC")
        };
        
        device.create_compute_pipeline(&ComputePipelineDescriptor{
            label: Some("Game compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        })
    }

    fn create_shadow_pipeline(device: &wgpu::Device, global_data_bgl: &BindGroupLayout, gamedata_bgl: &BindGroupLayout, _shader_type: ShaderType) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[global_data_bgl, gamedata_bgl],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shadow shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shaders/wgsl/shadow.wgsl").into())
        });
        
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 1,
                    },
                    VertexAttribute{
                        format: wgpu::VertexFormat::Float32x2,
                        offset: mem::size_of::<[f32; 2]>() as u64,
                        shader_location: 2,
                    }
                ],
            },
        ];

        device.create_render_pipeline(&RenderPipelineDescriptor{
            label: Some("Shadow pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "shadow",
                buffers: &buffer_layout
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,//Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: SHADOW_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview: None,
        })
    }

    fn create_depth_texture(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> wgpu::TextureView {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });

        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_shadow_texture(
        device: &wgpu::Device,
    ) -> (wgpu::TextureView, Sampler, BindGroupLayout, BindGroup) {
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: SHADOW_TEX_SIZE,
                height: SHADOW_TEX_SIZE,
                depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Shadow texture view"),
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("shadow bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            }],
        });

        let shadow_tex_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Shadow bind group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        (shadow_view, shadow_sampler, shadow_bind_group_layout, shadow_tex_bind_group)
    }

    fn create_buffers(device: &Device) -> (Buffer, Buffer, Buffer, Buffer) {
        let flap_pad_instance_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Flap pad instance data"),
            contents: bytemuck::cast_slice(&FlapPad::get_flaps_size_and_positions()),
            usage: BufferUsages::VERTEX,
        });
        const GAMEDATA_SIZE: usize = (FlapPad::RESOLUTION[0] * FlapPad::RESOLUTION[1] * (mem::size_of::<[f32; 2]>() as u32)) as usize;
        let gamedata = [-1e-7; GAMEDATA_SIZE];
        let gamedata_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Game data buffer"),
            contents: bytemuck::cast_slice(&gamedata),
            usage: BufferUsages::STORAGE,
        });

        let globals_buffer = device.create_buffer(&BufferDescriptor{
            label: Some("Globals buffer"),
            size: mem::size_of::<Globals>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let light_data_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Light data buffer"),
            //contents: bytemuck::cast_slice(&[LightData::new([1.5, 3.0, 3.0])]),
            contents: bytemuck::cast_slice(&[LightData::new([0.0, 0.0, 3.0])]),
            usage: BufferUsages::UNIFORM,
        });

        (flap_pad_instance_buffer, gamedata_buffer, globals_buffer, light_data_buffer)
    }

    // fn get_light_data(device: &Device) -> (BindGroupLayout, BindGroup) {
    //     // let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
    //     //     label: Some("Light data bgl"),
    //     //     entries: &[BindGroupLayoutEntry{
    //     //         binding: 0,
    //     //         visibility: ShaderStages::VERTEX_FRAGMENT,
    //     //         ty: wgpu::BindingType::Buffer {
    //     //             ty: wgpu::BufferBindingType::Uniform,
    //     //             has_dynamic_offset: false,
    //     //             min_binding_size: None
    //     //         },
    //     //         count: None,
    //     //     }],
    //     // });
    //     let light_data_buffer = device.create_buffer_init(&BufferInitDescriptor{
    //         label: Some("Light data buffer"),
    //         contents: bytemuck::cast_slice(&[LightData::new([0.0, 3.0, 3.0])]),
    //         usage: BufferUsages::UNIFORM,
    //     });
    //     let bg = device.create_bind_group(&BindGroupDescriptor{
    //         label: Some("Light data bindgroup"),
    //         layout: &bgl,
    //         entries: &[BindGroupEntry{
    //             binding: 0,
    //             resource: light_data_buffer.as_entire_binding(),
    //         }],
    //     });
        
    //     (bgl, bg)
    // }
}

struct FlapPad {
}

impl FlapPad {
    const WORLD_SIZE: [f32; 2] = [2.0, 1.0];
    const RESOLUTION: [u32; 2] = [32, 16];

    fn get_flaps_size_and_positions() -> Vec<f32>{
        let flap_size: [f32; 2] = [Self::WORLD_SIZE[0]/(Self::RESOLUTION[0] as f32), Self::WORLD_SIZE[1]/(Self::RESOLUTION[1] as f32)];
        let res: Vec<f32> = (0..Self::RESOLUTION[1]).flat_map(|y|{
            (0..Self::RESOLUTION[0]).flat_map(move |x|{
                let pos_x = -Self::WORLD_SIZE[0] + flap_size[0] + (x as f32) * 2.0 * flap_size[0];
                let pos_y = -Self::WORLD_SIZE[1] + flap_size[1] + (y as f32) * 2.0 * flap_size[1];
                vec![flap_size[0], flap_size[1], pos_x, pos_y].repeat(3)
            })
        }).collect();
        res
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    input_res:          [f32; 2],
    time:               f32,
    time_delta:         f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightData {
    view_proj:          [[f32; 4]; 4],
    position:           [f32; 4],
}

impl LightData {
    fn new(position: [f32; 3]) -> Self {
        let light_view_matrix = glm::look_at(&position.into(), &glm::Vec3::new(0.0, 0.0, 0.0), &glm::Vec3::new(0.0, 1.0, 0.0));
        let light_proj_matrix = glm::ortho(-2.5, 2.5, -1.5, 1.5, 0., 7.);

        LightData {
            view_proj: (light_proj_matrix * light_view_matrix).into(),
            position: [position[0], position[1], position[2], 18.0],
        }
    }
}