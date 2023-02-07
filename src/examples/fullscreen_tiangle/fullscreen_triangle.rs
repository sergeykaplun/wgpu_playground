use std::iter;
use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, Queue, include_spirv_raw, ShaderModule, ShaderModuleDescriptor, ShaderStages, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupEntry, util::DeviceExt, BindGroupDescriptor, BindGroup, Buffer};
use crate::{app::App, app::ShaderType};

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: Buffer,
    uniform_bindgroup: BindGroup,
    queue: Queue,
}

pub struct FullscreenTriangleExample {
    renderer : Renderer,
    resolution : Option<[u32; 2]>
}

impl App for FullscreenTriangleExample{
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType
    ) -> Self {
        let binding_0 = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("Fullscreen tri layout"),
            entries: &[BindGroupLayoutEntry{
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Full-screen triangle pipeline layout"),
                bind_group_layouts: &[&binding_0],
                push_constant_ranges: &[],
            }
        );

        let resolution = [sc.width, sc.height, 0, 0];
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[resolution]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Fullscreen triangle bindgroup"),
            layout: &binding_0,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let renderer = Renderer {
            pipeline: FullscreenTriangleExample::create_render_pipeline(device, &pipeline_layout, sc.format, shader_type),
            uniform_buffer,
            uniform_bindgroup,
            queue
        };

        Self{renderer, resolution: None}
    }

    fn resize(&mut self, sc: &wgpu::SurfaceConfiguration, _device: &wgpu::Device) {
        self.resolution = Some([sc.width, sc.height]);
    }

    fn tick(&mut self, _delta: f32) {}

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
            if let Some(res) = self.resolution {
                let resolution = [res[0], res[1], 0, 0];
                self.renderer.queue.write_buffer(&self.renderer.uniform_buffer, 0, bytemuck::cast_slice(&[resolution]));
                self.resolution = None;
            }
            
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
        
            render_pass.set_bind_group(0, &self.renderer.uniform_bindgroup, &[]);
            render_pass.set_pipeline(&self.renderer.pipeline);
            render_pass.draw(0..3, 0..1);
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn process_input(&mut self, event: &winit::event::WindowEvent) -> bool {
        false
    }
}

impl FullscreenTriangleExample {
    fn create_render_pipeline(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        tex_format: TextureFormat,
        shader_type: ShaderType
    ) -> wgpu::RenderPipeline {
        //TODO refactor this branching
        match shader_type {
            ShaderType::WGSL => {
                let shader_module = device.create_shader_module(ShaderModuleDescriptor{
                    label: Some("WGSL shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/fullscreen_tri.wgsl").into()),
                });
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("SimpleTriApp pipeline"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
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
                    fragment: Some(FragmentState {
                        module: &shader_module,
                        entry_point: "fs_main",
                        targets: &[Some(ColorTargetState {
                            format: tex_format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent::REPLACE,
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })]
                    }),
                    multiview: None,
                })
            },
            ShaderType::SPIRV => {
                let vs_shader_module: ShaderModule;
                let fs_shader_module: ShaderModule;
                unsafe {
                    fs_shader_module = device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/fullscreen_tri_fs.spv"));
                    vs_shader_module = device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/fullscreen_tri_vs.spv"));
                };
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("SimpleTriApp pipeline"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vs_shader_module,
                        entry_point: "main",
                        buffers: &[],
                    },
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
                    fragment: Some(FragmentState {
                        module: &fs_shader_module,
                        entry_point: "main",
                        targets: &[Some(ColorTargetState {
                            format: tex_format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })]
                    }),
                    multiview: None,
                })
            },
        }
    }
}