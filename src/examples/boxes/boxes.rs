use std::iter;

use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, VertexBufferLayout, VertexAttribute, util::{DeviceExt, BufferInitDescriptor}, BufferUsages, RenderPipeline, Queue, Buffer, ShaderModuleDescriptor, BindGroupLayout, include_spirv_raw, ShaderModule, VertexState, DepthStencilState, StencilState, DepthBiasState, RenderPassDepthStencilAttachment, Operations, TextureView};
use winit::event::WindowEvent;

use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}};

static CUBE_DATA: &'static [f32] = &[
    -1.0,-1.0,-1.0, -1.0,-1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0,-1.0, -1.0,-1.0,-1.0, -1.0, 1.0,-1.0,
     1.0,-1.0, 1.0, -1.0,-1.0,-1.0,  1.0,-1.0,-1.0, 1.0, 1.0,-1.0,  1.0,-1.0,-1.0, -1.0,-1.0,-1.0,
    -1.0,-1.0,-1.0, -1.0, 1.0, 1.0, -1.0, 1.0,-1.0, 1.0,-1.0, 1.0, -1.0,-1.0, 1.0, -1.0,-1.0,-1.0,
    -1.0, 1.0, 1.0, -1.0,-1.0, 1.0,  1.0,-1.0, 1.0, 1.0, 1.0, 1.0,  1.0,-1.0,-1.0,  1.0, 1.0,-1.0,
     1.0,-1.0,-1.0,  1.0, 1.0, 1.0,  1.0,-1.0, 1.0, 1.0, 1.0, 1.0,  1.0, 1.0,-1.0, -1.0, 1.0,-1.0,
     1.0, 1.0, 1.0, -1.0, 1.0,-1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0,  1.0,-1.0, 1.0
];

struct Renderer {
    queue: Queue,
    render_pipeline: RenderPipeline,
    depth_tex_view: TextureView,
    vertex_buffer: Buffer,
    vertex_count: u32,

    instance_buffer: Buffer,
}

pub struct BoxesExample {
    renderer: Renderer,
    camera: ArcballCamera,
}

impl App for BoxesExample {
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

        let render_pipeline = BoxesExample::create_pipeline(device, sc.format, &camera_bind_group_layout, shader_type);
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Vertices array buffer"),
            contents: bytemuck::cast_slice(CUBE_DATA),
            usage: BufferUsages::VERTEX,
        });

        let instances: Vec<f32> = (0..100).flat_map(|id|{
            let x = (id/10 - 5) as f32;
            let z = (id%10 - 5) as f32;
            vec![x * 3., 0.0, z * 3.]
        }).collect();

        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Instance array buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: BufferUsages::VERTEX,
        });
        
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 100., 7.);
        let depth_tex_view = Self::create_depth_texture(sc, device);

        let vertex_count = (CUBE_DATA.len()/3) as u32;
        Self { renderer: Renderer { queue, render_pipeline, depth_tex_view, vertex_buffer, vertex_count, instance_buffer}, camera }
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
        
            render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.renderer.instance_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            render_pass.set_pipeline(&self.renderer.render_pipeline);
            render_pass.draw(0..self.renderer.vertex_count, 0..100);
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn process_input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }

    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
    }
}

impl BoxesExample {
    fn create_pipeline(device: &wgpu::Device, tex_format: TextureFormat, cam_bgl: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
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
                label: Some("Full-screen triangle pipeline layout"),
                bind_group_layouts: &[cam_bgl],
                push_constant_ranges: &[],
            }
        );
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SimpleTriApp pipeline"),
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
}