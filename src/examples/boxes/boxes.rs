use std::iter;

use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, VertexBufferLayout, VertexAttribute, util::{DeviceExt, BufferInitDescriptor}, BufferUsages, RenderPipeline, Queue, Buffer, ShaderModuleDescriptor, BindGroupLayout, include_spirv_raw, ShaderModule, VertexState};
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
    vertex_buffer: Buffer,
    vertex_count: u32,
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
        
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 100., 7.);

        let vertex_count = (CUBE_DATA.len()/3) as u32;
        Self { renderer: Renderer { queue, render_pipeline, vertex_buffer, vertex_count, }, camera }
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
                depth_stencil_attachment: None
            });
        
            render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            render_pass.set_pipeline(&self.renderer.render_pipeline);
            render_pass.draw(0..self.renderer.vertex_count, 0..1);
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
        let vertex_buffer_layout = [VertexBufferLayout{
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute{
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }];
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
                    buffers: &vertex_buffer_layout,
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
                    buffers: &vertex_buffer_layout,
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
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(fragment_state),
            multiview: None,
        })
    }
}