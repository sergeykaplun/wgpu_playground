use std::iter;

use rand::{distributions::Uniform, prelude::Distribution};
use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, VertexBufferLayout, VertexAttribute, util::{DeviceExt, BufferInitDescriptor}, BufferUsages, RenderPipeline, Queue, Buffer, ShaderModuleDescriptor, BindGroupLayout, include_spirv_raw, ShaderModule, VertexState, DepthStencilState, StencilState, DepthBiasState, RenderPassDepthStencilAttachment, Operations, TextureView, Sampler, BindGroupDescriptor, BindGroupEntry, BindGroup, ComputePipelineDescriptor, PipelineLayoutDescriptor, ComputePipeline, Features, BufferDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, ShaderStages};
use winit::event::{WindowEvent, ElementState, VirtualKeyCode};

use crate::{app::{App, ShaderType, AppVariant}, camera::{ArcballCamera, Camera}};

static CUBE_DATA: &'static [f32] = &[
    // front
    -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
     1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
     1.0,  1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    -1.0,  1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    // back
    -1.0,  1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
     1.0,  1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
     1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
    // right
     1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0,  1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0,  1.0,  1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0, -1.0,  1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
    // left
    -1.0, -1.0,  1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0,  1.0,  1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0,  1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    // top
     1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    -1.0, 1.0,  1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
     1.0, 1.0,  1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    // bottom
     1.0, -1.0,  1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
    -1.0, -1.0,  1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
     1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
];

static CUBE_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
    8, 9, 10, 10, 11, 8,
    12, 13, 14, 14, 15, 12,
    16, 17, 18, 18, 19, 16,
    20, 21, 22, 22, 23, 20,
];

static FLOOR_DATA: &'static [f32] = &[
    -20.0, -1.0, -20.0, 0.0, 0.0,
    -20.0, -1.0,  20.0, 0.0, 1.0, 
     20.0, -1.0,  20.0, 1.0, 1.0,
     20.0, -1.0, -20.0, 1.0, 0.0,
];

static FLOOR_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
];

const SHADOW_TEX_SIZE: u32 = 1024u32;
const SHADOW_WORKGROUP_SIZE: u32 = 16u32;
const CELLS_CNT: u32 = 10u32;

struct GlobalConstants {
    shadow_res:         [f32; 2],
    light_position:     [f32; 2],
    light_color:        [f32; 3],
    time_in_flight:      f32,
    cells_cnt:          [f32; 2],
    unused:             [f32; 2]
}

struct Renderer {
    queue: Queue,
    depth_tex_view: TextureView,
    shadow_tex_bind_group: BindGroup,

    cube_render_pipeline: RenderPipeline,
    cube_vertex_buffer: Buffer,
    cube_index_buffer: Buffer,
    cube_index_count: u32,
    cube_instance_buffer: Buffer,
    cube_instances_count: u32,

    floor_render_pipeline: RenderPipeline,
    floor_vertex_buffer: Buffer,
    floor_index_buffer: Buffer,
    floor_index_count: u32,

    shadow_compute_pipeline: ComputePipeline,
    shadow_bind_group: BindGroup,
    //shadow_uniform_buf: Buffer,
    work_group_count: u32,

    global_constants_buffer: Buffer,
    global_constants_bind_group: BindGroup,
}

pub struct BoxesExample {
    renderer: Renderer,
    camera: ArcballCamera,
    constants: GlobalConstants,
    //time_in_flight: f32,
    light_controller: LightController,
}

impl App for BoxesExample {
    fn get_extra_device_features(app_variant: AppVariant) -> Features {
        let mut features = match app_variant.shader_type {
            ShaderType::WGSL => Features::empty(),
            ShaderType::SPIRV => Features::SPIRV_SHADER_PASSTHROUGH,
        };
        features |= Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        features
    }

    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: ShaderType
    ) -> Self {
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

        let global_constants_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("light_pos_bind_group_layout"),
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

        let shadow_tex_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false     //TODO enable
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    }
                ],
            label: Some("shadow_tex_bind_group_layout"),
        });

        let (tex_view, tex_sampler) = Self::create_shadow_texture(device);

        let cube_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("cube verices"),
            contents: bytemuck::cast_slice(CUBE_DATA),
            usage: BufferUsages::VERTEX,
        });
        let cube_index_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("cube indices"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: BufferUsages::INDEX,
        });
        
        let mut instances: Vec<f32> = (0..CELLS_CNT*CELLS_CNT).flat_map(|id|{
            let x = (id/CELLS_CNT * 4) as f32;
            let z = (id%CELLS_CNT * 4) as f32;
            vec![x - 18., z - 18., 0.0, 0.0]
        }).collect();
        let mut rng = rand::thread_rng();
        let unif = Uniform::new_inclusive(-1.0, 1.0);
        for cells_center in instances.chunks_mut(4) {
            cells_center[2] += unif.sample(&mut rng) * 1.5;
            cells_center[3] += unif.sample(&mut rng) * 1.5;
            
        };

        let cube_instance_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("vube instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        });
        let floor_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("floor verices"),
            contents: bytemuck::cast_slice(FLOOR_DATA),
            usage: BufferUsages::VERTEX,
        });
        let floor_index_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("floor indices"),
            contents: bytemuck::cast_slice(FLOOR_INDICES),
            usage: BufferUsages::INDEX,
        });
        
        let cube_render_pipeline = BoxesExample::create_boxes_rp(device, sc.format, &camera_bind_group_layout, &shadow_tex_bind_group_layout, &global_constants_bind_group_layout, shader_type);
        let floor_render_pipeline = BoxesExample::create_ground_rp(device, sc.format, &camera_bind_group_layout, &shadow_tex_bind_group_layout, &global_constants_bind_group_layout, shader_type);
        let (shadow_compute_pipeline, shadow_bind_group) = BoxesExample::create_shadow_cp(device, &tex_view, &cube_instance_buffer, &global_constants_bind_group_layout, shader_type);

        let shadow_tex_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("shadow_tex_bindgroup"),
            layout: &shadow_tex_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex_view),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&tex_sampler),
                }
            ],
        });

        let global_constants_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Global constants uniform uniform buffer"),
            size: std::mem::size_of::<GlobalConstants>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false 
        });
        let global_constants_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("global_constants_bind_group"),
            layout: &global_constants_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: global_constants_buffer.as_entire_binding(),
                }
            ],
        });

        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 100., 7., 35.);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        Self {
            renderer: Renderer {
                queue,
                depth_tex_view,
                shadow_tex_bind_group,

                cube_render_pipeline,
                cube_vertex_buffer,
                cube_index_buffer,
                cube_index_count: CUBE_INDICES.len() as u32,
                cube_instance_buffer,
                cube_instances_count: CELLS_CNT * CELLS_CNT,

                floor_render_pipeline,
                floor_vertex_buffer,
                floor_index_buffer,
                floor_index_count: FLOOR_INDICES.len() as u32,

                shadow_compute_pipeline,
                shadow_bind_group,
                //shadow_uniform_buf,
                work_group_count: SHADOW_TEX_SIZE/SHADOW_WORKGROUP_SIZE + 1, // div_ceil

                global_constants_buffer,
                global_constants_bind_group
            },
            camera,
            light_controller: LightController::new(0.1),
            constants: GlobalConstants {
                shadow_res: [SHADOW_TEX_SIZE as f32, SHADOW_TEX_SIZE as f32],
                light_position: [0.5; 2],
                light_color: [0.25, 0.5, 0.75],
                time_in_flight: 0.0,
                cells_cnt: [CELLS_CNT as f32; 2],
                unused: [0.0; 2]
            },
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
        
        encoder.push_debug_group("shadow pass");
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Shadow") });
            cpass.set_pipeline(&self.renderer.shadow_compute_pipeline);
            cpass.set_bind_group(0, &self.renderer.global_constants_bind_group, &[]);
            cpass.set_bind_group(1, &self.renderer.shadow_bind_group, &[]);
            cpass.dispatch_workgroups(self.renderer.work_group_count, self.renderer.work_group_count, 1);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("geometry render pass");
        {
            let globals = [self.constants.shadow_res[0], self.constants.shadow_res[1], 
                                      self.constants.light_position[0], self.constants.light_position[1],
                                      self.constants.light_color[0], self.constants.light_color[1], self.constants.light_color[2],
                                      self.constants.time_in_flight,
                                      self.constants.cells_cnt[0], self.constants.cells_cnt[1],
                                      0.0, 0.0];
            self.renderer.queue.write_buffer(&self.renderer.global_constants_buffer, 0, bytemuck::cast_slice(&globals));

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.02,
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

            render_pass.set_pipeline(&self.renderer.floor_render_pipeline);
            render_pass.set_vertex_buffer(0, self.renderer.floor_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.renderer.global_constants_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.renderer.shadow_tex_bind_group, &[]);
            render_pass.set_index_buffer(self.renderer.floor_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.renderer.floor_index_count, 0, 0..1);

            render_pass.set_pipeline(&self.renderer.cube_render_pipeline);
            render_pass.set_vertex_buffer(0, self.renderer.cube_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.renderer.cube_instance_buffer.slice(..));
            render_pass.set_index_buffer(self.renderer.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.renderer.cube_index_count, 0, 0..self.renderer.cube_instances_count);
        }
        encoder.pop_debug_group();
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.light_controller.input(event);
        self.camera.input(event)
    }

    fn tick(&mut self, delta: f32) {
        self.light_controller.tick(delta);
        self.constants.light_position = self.light_controller.light_position;
        self.camera.tick(delta, &self.renderer.queue);
        self.constants.time_in_flight += delta;
    }
}

impl BoxesExample {
    fn create_boxes_rp(device: &wgpu::Device, tex_format: TextureFormat, cam_bgl: &BindGroupLayout, shadow_tex_bgl: &BindGroupLayout, constants_bgl: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                }],
            },
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3,
                }],
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
        let mut spirv_modules : Vec<ShaderModule> = vec![];

        let vertex_state: VertexState;
        let fragment_state: FragmentState;
        match shader_type {
            ShaderType::WGSL => {
                spirv_modules.push(device.create_shader_module(ShaderModuleDescriptor{
                    label: Some("WGSL shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/boxes.wgsl").into()),
                }));
                vertex_state = wgpu::VertexState {
                    module: &spirv_modules[0],
                    entry_point: "vs_main",
                    buffers: &buffer_layout,
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[0],
                    entry_point: "fs_main",
                    targets: &color_states
                }
            },
            ShaderType::SPIRV => {
                unsafe {
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/boxes.vs.spv")));
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/boxes.fs.spv")));
                };
                vertex_state = wgpu::VertexState {
                    module: &spirv_modules[0],
                    entry_point: "main",
                    buffers: &buffer_layout,
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[1],
                    entry_point: "main",
                    targets: &color_states
                }
            },
        }
        
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Boxes pipeline layout"),
                bind_group_layouts: &[constants_bgl, cam_bgl, &shadow_tex_bgl],
                //bind_group_layouts: &[cam_bgl, &light_bgl],
                push_constant_ranges: &[],
            }
        );
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Boxes pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(Face::Back),
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
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        })
        
    }

    fn create_ground_rp(device: &wgpu::Device, tex_format: TextureFormat, cam_bgl: &BindGroupLayout, shadow_bgl: &BindGroupLayout, constants_bgl: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                }],
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
        let mut spirv_modules : Vec<ShaderModule> = vec![];

        let vertex_state: VertexState;
        let fragment_state: FragmentState;
        match shader_type {
            ShaderType::WGSL => {
                spirv_modules.push(device.create_shader_module(ShaderModuleDescriptor{
                    label: Some("WGSL shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/ground.wgsl").into()),
                }));
                vertex_state = wgpu::VertexState {
                    module: &spirv_modules[0],
                    entry_point: "vs_main",
                    buffers: &buffer_layout,
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[0],
                    entry_point: "fs_main",
                    targets: &color_states
                }
            },
            ShaderType::SPIRV => {
                unsafe {
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/ground.vs.spv")));
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/ground.fs.spv")));
                };
                vertex_state = wgpu::VertexState {
                    module: &spirv_modules[0],
                    entry_point: "main",
                    buffers: &buffer_layout,
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[1],
                    entry_point: "main",
                    targets: &color_states
                }
            },
        }
        
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Floor pipeline layout"),
                bind_group_layouts: &[constants_bgl, cam_bgl, shadow_bgl],
                push_constant_ranges: &[],
            }
        );
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Floor pipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(Face::Back),
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
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        })
    }

    fn create_shadow_cp(device: &wgpu::Device, tex_view: &TextureView, instance_buffer: &Buffer, shadow_bgl: &BindGroupLayout, shader_type: ShaderType) -> (wgpu::ComputePipeline, BindGroup) {
        let shadow_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    }
                ],
            label: Some("camera_bind_group_layout"),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Shadow pipeline descriptor"),
            bind_group_layouts: &[shadow_bgl, &shadow_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let shader_module = match shader_type {
            ShaderType::WGSL => device.create_shader_module(ShaderModuleDescriptor{
                                    label: Some("WGSL shader"),
                                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/shadow.wgsl").into()),
                                }),
            ShaderType::SPIRV => unsafe {
                                    device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/shadow.cs.spv"))
                                },
        };
        
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor{
            label: Some("Shadow pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Shadow pass bindgroup"),
            layout: &shadow_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(tex_view),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: instance_buffer.as_entire_binding(),
                }
            ],
        });
        (pipeline, bind_group)
    }
}

impl BoxesExample {
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

    fn create_shadow_texture(device: &wgpu::Device) -> (TextureView, Sampler) {
        let size = SHADOW_TEX_SIZE;
        let texture_extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        (texture_view, sampler)
    }
}

struct LightController {
    light_position: [f32; 2],
    directions_pressed: [bool; 4],
    speed: f32,
}

impl LightController {
    const LEFT: usize = 0; const UP: usize = 1; const RIGHT: usize = 2; const DOWN: usize = 3;

    fn new(speed: f32) -> Self {
        Self {
            light_position: [0.5, 0.5],
            directions_pressed: [false, false, false, false],
            speed,
        }
    }

    fn input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                input,
                ..
            } => {
                let index = match input.virtual_keycode.unwrap() {
                    VirtualKeyCode::A | VirtualKeyCode::Left => Some(Self::LEFT),
                    VirtualKeyCode::W | VirtualKeyCode::Up => Some(Self::UP),
                    VirtualKeyCode::D | VirtualKeyCode::Right => Some(Self::RIGHT),
                    VirtualKeyCode::S | VirtualKeyCode::Down => Some(Self::DOWN),
                    _ => None
                };
                if let Some(ind) = index {
                    self.directions_pressed[ind] = input.state == ElementState::Pressed;
                }
            },
            _ => {}
        }
    }

    fn tick(&mut self, time_delta: f32) {
        if self.directions_pressed[Self::LEFT] {
            self.light_position[0] -= time_delta * self.speed;
        }
        if self.directions_pressed[Self::RIGHT] {
            self.light_position[0] += time_delta * self.speed;
        }
        if self.directions_pressed[Self::UP] {
            self.light_position[1] -= time_delta * self.speed;
        }
        if self.directions_pressed[Self::DOWN] {
            self.light_position[1] += time_delta * self.speed;
        }
    }
}