use std::{iter, mem};

use wgpu::{Queue, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, VertexState, FragmentState, ShaderModule, PrimitiveState, Face, DepthStencilState, StencilState, DepthBiasState, MultisampleState, ShaderModuleDescriptor, RenderPipeline, RenderPassDepthStencilAttachment, Operations, TextureView, RenderPipelineDescriptor, Sampler, BindGroup, Buffer, BindGroupLayout, BindGroupDescriptor, BindGroupLayoutDescriptor, BindingType, BindGroupEntry};
use winit::event::WindowEvent;

use crate::{app::{App, ShaderType}, assets_helper::{self, Mesh}, camera::{ArcballCamera, Camera}};
struct Renderer {
    queue: Queue,
    
    pipeline: RenderPipeline,
    depth_tex_view: TextureView,

    shadow_pipeline: RenderPipeline,
    shadow_texture_view: TextureView,
    shadow_tex_bind_group: BindGroup,
    
    light_buffer: Buffer,
    light_bind_group: BindGroup,
}

pub struct ShadowMappingExample {
    renderer: Renderer,
    meshes: Vec<Mesh>,
    camera: ArcballCamera,
    time_in_flight: f32,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SHADOW_TEX_SIZE: u32 = 512u32;


impl App for ShadowMappingExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: ShaderType
    ) -> Self {
        let meshes = pollster::block_on(
            assets_helper::load_model(
                "human_floor.obj",
                &device,
            )
        ).expect("Error while loading model");

        let (light_bind_group_layout, light_bind_group, light_buffer) = {
            let light_uniform_size = mem::size_of::<LightData>() as wgpu::BufferAddress;
            let light_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: light_uniform_size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let light_bind_group_layout = device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(light_uniform_size),
                        },
                        count: None,
                    }],
                }
            );
    
            let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &light_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: light_buf.as_entire_binding(),
                }],
                label: None,
            });
            (light_bind_group_layout, light_bind_group, light_buf)
        };

        let (shadow_texture_view, shadow_sampler) = Self::create_shadow_texture(device);
        let (pipeline, shadow_tex_bind_group) = Self::create_output_pipeline(&device, sc.format, &light_bind_group_layout, &shadow_texture_view, &shadow_sampler, shader_type);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        let shadow_pipeline = Self::create_shadow_pipeline(device, &light_bind_group_layout, shader_type);
        
        let renderer = Renderer { queue, pipeline, depth_tex_view, shadow_pipeline, light_bind_group, shadow_texture_view, shadow_tex_bind_group, light_buffer };
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 200., 7., 35.);
        Self{ renderer, meshes, camera, time_in_flight: 0.0 }
    }

    fn render(&mut self, surface: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError> {
        let light_data = Self::get_light_matrix(self.time_in_flight);
        self.renderer.queue.write_buffer(&self.renderer.light_buffer, 0, bytemuck::cast_slice(&[light_data]));
        
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        encoder.insert_debug_marker("shadow pass");
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.renderer.shadow_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            pass.set_pipeline(&self.renderer.shadow_pipeline);
            pass.set_bind_group(0, &self.renderer.light_bind_group, &[]);

            for mesh in &self.meshes {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
            }
        }
        encoder.pop_debug_group();

        {
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
        
            render_pass.set_pipeline(&self.renderer.pipeline);
            render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.renderer.light_bind_group, &[]);
            render_pass.set_bind_group(2, &self.renderer.shadow_tex_bind_group, &[]);
            for mesh in &self.meshes {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
            }
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
        self.time_in_flight += delta;
    }

}

impl ShadowMappingExample {
    fn create_output_pipeline(device: &wgpu::Device, tex_format: TextureFormat, light_bind_group_layout: &BindGroupLayout, shadow_tex_view: &TextureView, shadow_sampler: &Sampler, shader_type: ShaderType) -> (wgpu::RenderPipeline, BindGroup) {
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
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/geometry.wgsl").into()),
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
            _ => panic!()
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
                    resource: wgpu::BindingResource::TextureView(shadow_tex_view),
                },
                BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(shadow_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Output pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout, &shadow_bind_group_layout],
                push_constant_ranges: &[],
            }
        );
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            depth_stencil: Some(DepthStencilState{
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(fragment_state),
            multiview: None,
        });

        (pipeline, shadow_tex_bind_group)
    }

    fn create_shadow_pipeline(device: &wgpu::Device, light_bind_group_layout: &BindGroupLayout, _shader_type: ShaderType) -> wgpu::RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[light_bind_group_layout],
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
            }
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
                cull_mode: Some(wgpu::Face::Back),
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
    ) -> (wgpu::TextureView, Sampler) {
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

        (shadow_view, shadow_sampler)
    }

    fn get_light_matrix(time: f32) -> LightData {
        let distance = 10.;

        let light_position = glm::Vec3::new(time.sin() * distance, 10., time.cos() * distance);
        let light_view_matrix = glm::look_at(&light_position, &glm::Vec3::new(0.0, 0.0, 0.0), &glm::Vec3::new(0.0, 1.0, 0.0));
        let light_proj_matrix = glm::ortho(-10., 10., -10., 10., -100., 100.);
        
        LightData {
            view_proj: (light_proj_matrix * light_view_matrix).into(),
            position: glm::Vec4::new(light_position.x, light_position.y, light_position.z, 0.0).into()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightData {
    view_proj: [[f32; 4]; 4],
    position: [f32; 4]
}