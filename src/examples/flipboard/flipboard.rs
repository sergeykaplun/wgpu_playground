use std::{iter, mem};

use wgpu::{Queue, RenderPipeline, ColorTargetState, TextureFormat, ShaderModule, VertexState, FragmentState, Device, ShaderModuleDescriptor, PipelineLayoutDescriptor, PrimitiveState, MultisampleState, TextureView, ComputePipelineDescriptor, BindGroupEntry, BindGroupDescriptor, ComputePipeline, BindGroup, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages, util::{BufferInitDescriptor, DeviceExt}, BufferUsages, Buffer, VertexBufferLayout, VertexAttribute, BufferDescriptor, BindGroupLayout, RenderPassDepthStencilAttachment, Operations, DepthStencilState, StencilState, DepthBiasState, BufferAddress};
use winit::event::WindowEvent;

use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}};

static FLIP_PAD_DATA: &'static [f32] = &[
    -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    -1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
     1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
     1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
];

static FLIP_PAD_INDICES: &[u16] = &[
    0, 2, 1, 2, 0, 3
];

const GAME_TEXTURE_SIZE: u32 = 32;
const GAME_WORKGROUP_SIZE: u32 = 16u32;

struct Renderer {
    queue: Queue,
    depth_tex_view: TextureView,

    globals_buffer: Buffer,
    globals_bindgroup: BindGroup,

    gamedata_write_bindgroup: BindGroup,
    gamedata_read_bindgroup: BindGroup,
    
    flaps_pipeline: wgpu::RenderPipeline,
    game_compute_pipeline: wgpu::ComputePipeline,
    game_compute_workgroups_count: u32,

    flap_pad_vb: Buffer,
    flap_pad_ib: Buffer,
    flap_pad_index_cnt: u32,
    flap_pad_instance_buffer: Buffer,
    flap_pad_instances_cnt: u32,
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
        let globals_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("Globals bgl"),
            entries: &[BindGroupLayoutEntry{
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT | ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None,
            }],
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
        let (flap_pad_vb, flap_pad_ib, flap_pad_instance_buffer, gamedata_buffer, globals_buffer) = Self::create_buffers(device);
        let depth_tex_view = Self::create_depth_texture(sc, device);

        let game_compute_pipeline = Self::create_compute_pipeline(device, &globals_bgl, &game_buffer_write_bgl, shader_type);
        let flaps_pipeline = Self::create_render_pipeline(device, sc.format, &globals_bgl, &game_buffer_read_bgl, shader_type);
        
        let globals_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Global bg"),
            layout: &globals_bgl,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
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

        let renderer = Renderer {
            queue,
            depth_tex_view,
            
            globals_buffer,
            globals_bindgroup,
            //gamedata_buffer,
            gamedata_write_bindgroup,
            gamedata_read_bindgroup,

            flaps_pipeline,
            game_compute_pipeline,
            game_compute_workgroups_count: GAME_TEXTURE_SIZE/GAME_WORKGROUP_SIZE,

            flap_pad_vb,
            flap_pad_ib,
            flap_pad_index_cnt: FLIP_PAD_INDICES.len() as u32,

            flap_pad_instance_buffer,
            flap_pad_instances_cnt: GAME_TEXTURE_SIZE * GAME_TEXTURE_SIZE * 3
        };
        Self {
            renderer,
            globals: Globals {
                output_res: [sc.width as f32, sc.height as f32],
                input_res: [GAME_TEXTURE_SIZE as f32; 2],
                time: 0.0,
                time_delta: 0.0,
                _unused: [0.0; 2],
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
            cpass.set_bind_group(0, &self.renderer.globals_bindgroup, &[]);
            cpass.set_bind_group(1, &self.renderer.gamedata_write_bindgroup, &[]);
            cpass.dispatch_workgroups(self.renderer.game_compute_workgroups_count, self.renderer.game_compute_workgroups_count, 1);
        }
        encoder.pop_debug_group();
        
        {
            self.renderer.queue.write_buffer(&self.renderer.globals_buffer, 0, bytemuck::cast_slice(&self.globals.get()));

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
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
            render_pass.set_vertex_buffer(0, self.renderer.flap_pad_vb.slice(..));
            render_pass.set_vertex_buffer(1, self.renderer.flap_pad_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.renderer.flap_pad_ib.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.renderer.globals_bindgroup, &[]);
            render_pass.set_bind_group(1, &self.renderer.gamedata_read_bindgroup, &[]);
            render_pass.set_bind_group(2, &self.camera.camera_bind_group, &[]);
            render_pass.draw_indexed(0..self.renderer.flap_pad_index_cnt, 0, 0..self.renderer.flap_pad_instances_cnt);
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();
        
        self.globals.time_delta = 0.0;
        Ok(())
    }
    
    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
        self.globals.time += delta;
        self.globals.time_delta += delta;
    }

    fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }
}

impl FlipboardExample {
    fn create_render_pipeline(device: &Device, tex_format: TextureFormat, globals_bgl: &BindGroupLayout, gamedata_bgl: &BindGroupLayout, shader_type: ShaderType) -> RenderPipeline {
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
        
        let camera_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            label: Some("camera_bind_group_layout"),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Output pipeline"),
            bind_group_layouts: &[globals_bgl, &gamedata_bgl, &camera_bind_group_layout],
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
                format: Self::DEPTH_FORMAT,
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

    // fn create_texture(device: &Device) -> (TextureView, Sampler) {
    //     let texture_desc = wgpu::TextureDescriptor {
    //         size: wgpu::Extent3d {
    //             width: GAME_TEXTURE_SIZE,
    //             height: GAME_TEXTURE_SIZE,
    //             depth_or_array_layers: 1,
    //         },
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         dimension: wgpu::TextureDimension::D2,
    //         format: TextureFormat::R32Float,
    //         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
    //         label: None,
    //         view_formats: &[],
    //     };
    //     let texture = device.create_texture(&texture_desc);
    //     let mut descriptor = TextureViewDescriptor::default();
    //     descriptor.label = Some("Intermediate texture");
    //     let tex_view = texture.create_view(&descriptor);
    //     let tex_sampler = device.create_sampler(&SamplerDescriptor::default());
    //     (tex_view, tex_sampler)
    // }

    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
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
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });

        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_buffers(device: &Device) -> (Buffer, Buffer, Buffer, Buffer, Buffer) {
        let flap_pad_vb = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Flap pad vertices"),
            contents: bytemuck::cast_slice(FLIP_PAD_DATA),
            usage: BufferUsages::VERTEX,
        });
        let flap_pad_ib = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Flap pad vertices"),
            contents: bytemuck::cast_slice(FLIP_PAD_INDICES),
            usage: BufferUsages::INDEX,
        });
        
        let flap_size = 1./(GAME_TEXTURE_SIZE as f32);
        let instance_data: Vec<f32> = (0..GAME_TEXTURE_SIZE).flat_map(|y|{
            (0..GAME_TEXTURE_SIZE).flat_map(move |x|{
                (0..3).flat_map(move |id|{
                    let pos_x = -1.0 + flap_size + (x as f32) * 2. * flap_size;
                    let mut pos_y = -1.0 + flap_size + (y as f32) * 2. * flap_size;
                    if id == 1 {
                        pos_y -= flap_size;
                    }
                    vec![flap_size, flap_size, pos_x, pos_y]
                })
            })
        }).collect();

        let flap_pad_instance_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Flap pad instance data"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: BufferUsages::VERTEX,
        });
        const GAMEDATA_SIZE: usize = (GAME_TEXTURE_SIZE * GAME_TEXTURE_SIZE * (mem::size_of::<[f32; 2]>() as u32)) as usize;
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

        (flap_pad_vb, flap_pad_ib, flap_pad_instance_buffer, gamedata_buffer, globals_buffer)
    }
}

struct Globals {
    output_res:         [f32; 2],
    input_res:          [f32; 2],
    time:               f32,
    time_delta:         f32,
    _unused:             [f32; 2]
}

impl Globals {
    fn get(&self) -> Vec<f32> {
        vec![self.output_res[0], self.output_res[1],
             self.input_res[0], self.input_res[1],
             self.time,
             self.time_delta,
             0.0, 0.0]
    }
}