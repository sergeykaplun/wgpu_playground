use std::iter;
use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, Queue, include_spirv_raw, ShaderModule, ShaderModuleDescriptor};
use crate::{app::App, app::ShaderType};

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    queue: Queue,
}

pub struct SimpleTriApp {
    renderer : Renderer,
}

impl App for SimpleTriApp{
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Full-screen triangle pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            }
        );

        let renderer = Renderer {
            pipeline: SimpleTriApp::create_render_pipeline(device, &pipeline_layout, sc.format, shader_type),
            queue
        };

        Self{renderer}
    }

    fn resize(&mut self, _sc: &wgpu::SurfaceConfiguration, _device: &wgpu::Device) {}

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
        
            render_pass.set_pipeline(&self.renderer.pipeline);
            render_pass.draw(0..3, 0..1);
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl SimpleTriApp {
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
        }
    }
}