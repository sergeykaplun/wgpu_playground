use std::iter;

use wgpu::{Queue, RenderPipeline, ColorTargetState, TextureFormat, ShaderModule, VertexState, FragmentState, Device, ShaderModuleDescriptor, PipelineLayoutDescriptor, PrimitiveState, Face, MultisampleState, TextureView, TextureViewDescriptor, ComputePipelineDescriptor, BindGroupEntry, BindGroupDescriptor, ComputePipeline, BindGroup};

use crate::app::{App, ShaderType};

struct Renderer {
    queue: Queue,

    pipeline: wgpu::RenderPipeline,
    
    game_output_pipeline: wgpu::ComputePipeline,
    game_output_bind_group: BindGroup,
    game_output_workgroup_size: u32,
}

pub(crate) struct FlipboardExample {
    renderer: Renderer,
}

impl App for FlipboardExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: crate::app::ShaderType
    ) -> Self {
        let (game_output_pipeline, game_output_bind_group) = Self::create_game_pipeline(device, shader_type);
        let renderer = Renderer {
            queue,
            pipeline: Self::create_render_pipeline(device, sc.format, shader_type),
            game_output_pipeline,
            game_output_bind_group,
            game_output_workgroup_size: 16,
            //intermediate_texture_view: Self::create_texture(device, sc.format)
        };
        Self { renderer }
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
        let ops = wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.9,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            }),
            store: true,
        };

        encoder.push_debug_group("game output pass");
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Game output") });
            cpass.set_pipeline(&self.renderer.game_output_pipeline);
            cpass.set_bind_group(0, &self.renderer.game_output_bind_group, &[]);
            cpass.dispatch_workgroups(self.renderer.game_output_workgroup_size, self.renderer.game_output_workgroup_size, 1);
        }
        encoder.pop_debug_group();
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: ops,
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

impl FlipboardExample {
    fn create_render_pipeline(device: &Device, tex_format: TextureFormat, shader_type: ShaderType) -> RenderPipeline {
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
                    buffers: &[],
                };
                fragment_state = FragmentState {
                    module: &spirv_modules[0],
                    entry_point: "fs_main",
                    targets: &color_states
                }
            },
            // ShaderType::SPIRV => {
            //     unsafe {
            //         spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/fullscreen_tri_vs.spv")));
            //         spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/fullscreen_tri_fs.spv")));
            //     };
            //     vertex_state = wgpu::VertexState {
            //         module: &spirv_modules[0],
            //         entry_point: "main",
            //         buffers: &[],
            //     };
            //     fragment_state = FragmentState {
            //         module: &spirv_modules[1],
            //         entry_point: "main",
            //         targets: &color_states
            //     }
            // },
            _ => panic!("")
        }

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Output pipeline"),
            bind_group_layouts: &[],
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

    fn create_game_pipeline(device: &Device, shader_type: ShaderType) -> (ComputePipeline, BindGroup) {
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
                    }
                ],
            label: Some("game_output_bind_group_layout"),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Game output pipeline descriptor"),
            bind_group_layouts: &[&shadow_bind_group_layout],
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
        
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor{
            label: Some("Game compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });
        let texture_view = Self::create_texture(device);
        let bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Game compute bindgroup"),
            layout: &shadow_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                }
            ],
        });
        (pipeline, bind_group)
    }

    fn create_texture(device: &Device) -> TextureView {
        let texture_size = 256u32;

        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: texture_size,
                height: texture_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_desc);
        let mut descriptor = TextureViewDescriptor::default();
        descriptor.label = Some("Intermediate texture");
        texture.create_view(&descriptor)
    }
}