use std::iter;

use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, VertexBufferLayout, VertexAttribute, util::{DeviceExt, BufferInitDescriptor}, BufferUsages, RenderPipeline, Queue, Buffer, ShaderModuleDescriptor, BindGroupLayout, include_spirv_raw, ShaderModule, VertexState, DepthStencilState, StencilState, DepthBiasState, RenderPassDepthStencilAttachment, Operations, TextureView, Sampler, BindGroupDescriptor, BindGroupEntry, BindGroup, ComputePipelineDescriptor, PipelineLayoutDescriptor, ComputePipeline, Features, BufferDescriptor};
use winit::event::WindowEvent;

use crate::{app::{App, ShaderType, AppVariant}, camera::{ArcballCamera, Camera}};

static CUBE_DATA: &'static [f32] = &[
    -1.0, -1.0, 1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, 0.0, 1.0,
    -1.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, -1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 0.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0,
     1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
    -1.0, -1.0, 1.0, 1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 0.0, -1.0, 1.0, -1.0, 0.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0,
     1.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, -1.0, 0.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
     1.0, -1.0, 1.0, 0.0, 0.0, -1.0, -1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0,
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

const SHADOW_TEX_SIZE: u32 = 512u32;
const SHADOW_WORKGROUP_SIZE: u32 = 16u32;

struct Renderer {
    queue: Queue,
    depth_tex_view: TextureView,
    floor_tex_bind_group: BindGroup,

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
    shadow_uniform_buf: Buffer,
    work_group_count: u32,
}

pub struct BoxesExample {
    renderer: Renderer,
    camera: ArcballCamera,
    time_in_flight: f32,
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

        let floor_tex_bind_group_layout = device.create_bind_group_layout(
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
            label: Some("camera_bind_group_layout"),
        });

        let (tex_view, tex_sampler) = Self::create_shadow_texture(device);

        let cube_render_pipeline = BoxesExample::create_cube_rp(device, sc.format, &camera_bind_group_layout, shader_type);
        let floor_render_pipeline = BoxesExample::create_floor_rp(device, sc.format, &camera_bind_group_layout, &floor_tex_bind_group_layout, shader_type);
        let (shadow_compute_pipeline, shadow_bind_group, shadow_uniform_buf) = BoxesExample::create_shadow_cp(device, &tex_view, shader_type);

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
        let instances: Vec<f32> = (0..100).flat_map(|id|{
            let x = (id/10 * 4) as f32;
            let z = (id%10 * 4) as f32;
            vec![x - 18., 0.0, z - 18., 0.0]
        }).collect();

        let cube_instance_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("vube instances"),
            contents: bytemuck::cast_slice(&instances),
            usage: BufferUsages::VERTEX,
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
        let floor_tex_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("floor_tex_bindgroup"),
            layout: &floor_tex_bind_group_layout,
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

        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 100., 7.);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        Self {
            renderer: Renderer {
                queue,
                depth_tex_view,
                floor_tex_bind_group,

                cube_render_pipeline,
                cube_vertex_buffer,
                cube_index_buffer,
                cube_index_count: CUBE_INDICES.len() as u32,
                cube_instance_buffer,
                cube_instances_count: 100,

                floor_render_pipeline,
                floor_vertex_buffer,
                floor_index_buffer,
                floor_index_count: FLOOR_INDICES.len() as u32,

                shadow_compute_pipeline,
                shadow_bind_group,
                shadow_uniform_buf,
                work_group_count: SHADOW_TEX_SIZE/SHADOW_WORKGROUP_SIZE + 1, // div_ceil
            },
            camera,
            time_in_flight: 0.0,
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
            let buf = [SHADOW_TEX_SIZE as f32, SHADOW_TEX_SIZE as f32, self.time_in_flight, 0.0];
            self.renderer.queue.write_buffer(&self.renderer.shadow_uniform_buf, 0, bytemuck::cast_slice(&[buf]));

            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Shadow") });
            cpass.set_pipeline(&self.renderer.shadow_compute_pipeline);
            cpass.set_bind_group(0, &self.renderer.shadow_bind_group, &[]);
            cpass.dispatch_workgroups(self.renderer.work_group_count, self.renderer.work_group_count, 1);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("geometry render pass");
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.9,
                            g: 0.2,
                            b: 0.3,
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
            render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer.floor_tex_bind_group, &[]);
            render_pass.set_index_buffer(self.renderer.floor_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.renderer.floor_index_count, 0, 0..1);

            // render_pass.set_pipeline(&self.renderer.cube_render_pipeline);
            // render_pass.set_vertex_buffer(0, self.renderer.cube_vertex_buffer.slice(..));
            // render_pass.set_vertex_buffer(1, self.renderer.cube_instance_buffer.slice(..));
            // render_pass.set_index_buffer(self.renderer.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            // render_pass.draw_indexed(0..self.renderer.cube_index_count, 0, 0..self.renderer.cube_instances_count);
        }
        encoder.pop_debug_group();
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }

    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
        self.time_in_flight += delta;
    }
}

impl BoxesExample {
    fn create_cube_rp(device: &wgpu::Device, tex_format: TextureFormat, cam_bgl: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
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
            },
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 2,
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
                bind_group_layouts: &[cam_bgl],
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

    fn create_floor_rp(device: &wgpu::Device, tex_format: TextureFormat, cam_bgl: &BindGroupLayout, tex_bgl: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
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
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/floor.wgsl").into()),
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
                //TODO error here
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
                label: Some("Floor pipeline layout"),
                bind_group_layouts: &[cam_bgl, tex_bgl],
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

    fn create_shadow_cp(device: &wgpu::Device, tex_view: &TextureView, shader_type: ShaderType) -> (wgpu::ComputePipeline, BindGroup, Buffer) {
        let shadow_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R32Float,
                            view_dimension: wgpu::TextureViewDimension::D2
                        },
                        count: None,
                    }
                ],
            label: Some("camera_bind_group_layout"),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Shadow pipeline descriptor"),
            bind_group_layouts: &[&shadow_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader_module = device.create_shader_module(ShaderModuleDescriptor{
            label: Some("WGSL shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/shadow.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor{
            label: Some("Shadow pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });
        let buf = device.create_buffer(&BufferDescriptor {
            label: Some("Shadow pass uniform"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false }
        );
        let bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Shadow pass bindgroup"),
            layout: &shadow_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(tex_view),
                }
            ],
        });
        (pipeline, bind_group, buf)
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
    /*
    fn hash3(p: glm::Vec2) -> glm::Vec3 {
        let q = glm::Vec3::new(
            p.dot(&glm::Vec2::new(127.1,311.7)),
            p.dot(&glm::Vec2::new(269.5,183.3)),
            p.dot(&glm::Vec2::new(419.2,371.9)));
        fract(&(glm::sin(&q) * 43758.5453))
    }

    fn hash( p: glm::Vec2 ) -> glm::Vec2 {
        let res = glm::Vec2::new( p.dot(&glm::Vec2::new(127.1,311.7)),p.dot(&glm::Vec2::new(269.5,183.3)) );
        fract(&(glm::sin(&res) * 43758.5453123)) * 2.0 - glm::Vec2::new(1.0, 1.0)
    }

    fn voronoi(uv: glm::Vec2) -> glm::Vec3{
        let n = floor(&uv);
        let f = glm::fract(&uv);

        let mut mg = glm::Vec2::zeros();
        let mut mr = glm::Vec2::zeros();

        let mut md = 8.0f32;
        (-1..1).for_each(|j|{
            (-1..1).for_each(|i|{
                let neighbour = n + glm::Vec2::new(i as f32,j as f32);
                let neighbour_center = neighbour + glm::Vec2::new(0.5, 0.5);
                let d = neighbour_center.metric_distance(&(n + f));

                if d < md {
                    md = d;
                    //mr = r;
                    //mg = g;
                }
            });
        });
        //return glm::Vec3::new( md, mr.x, mr.y );
        return glm::Vec3::new(md, md, md);
    }

    fn create_voronoi_texture(device: &wgpu::Device, queue: &Queue) -> (TextureView, Sampler) {
        let size = 256u32;
        let texels: Vec<f32> = (0..size * size).flat_map(|id| {
            let uv = glm::Vec2::new((id/256) as f32, (id%256) as f32) / 256.0 * 10.;
            let v = Self::voronoi(uv);
            [v.x, v.y, v.z, 1.]
        }).collect();
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
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        queue.write_texture(
            texture.as_image_copy(),
            &bytemuck::cast_slice(&texels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(size * 16),
                rows_per_image: None,
            },
            texture_extent,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        (texture_view, sampler)
    }
    */
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
