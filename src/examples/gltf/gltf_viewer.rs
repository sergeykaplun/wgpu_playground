use std::{iter, mem};

use wgpu::{Queue, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, VertexState, FragmentState, ShaderModule, PrimitiveState, Face, DepthStencilState, StencilState, DepthBiasState, MultisampleState, ShaderModuleDescriptor, RenderPipeline, RenderPassDepthStencilAttachment, Operations, TextureView, BindGroup, Buffer, BindGroupLayout};
//use winit::event::WindowEvent;

use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}, model::{GLTFModel, Drawable, NOD_MM_BGL, MATERIAL_BGL, parse_gltf}, assets_helper::ResourceManager, input_event::InputEvent};
struct Renderer {
    queue: Queue,
    
    pipeline: RenderPipeline,
    depth_tex_view: TextureView,

    light_buffer: Buffer,
    light_bind_group: BindGroup,
}

pub struct GLTFViewerExample {
    renderer: Renderer,
    model: GLTFModel,
    camera: ArcballCamera,
    time_in_flight: f32,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl<T: ResourceManager> App<T> for GLTFViewerExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: ShaderType,
        resource_manager: &T
    ) -> Self {
        //let model = pollster::block_on(parse_gltf("models/FlightHelmet/glTF/FlightHelmet.gltf", &device, &queue, resource_manager));
        let model = pollster::block_on(parse_gltf("models/DamagedHelmet/glTF-Embedded/DamagedHelmet.gltf", &device, &queue, resource_manager));
        
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
                        visibility: wgpu::ShaderStages::FRAGMENT,
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

        //let pipeline = Self::create_output_pipeline(&device, sc.format, &light_bind_group_layout, shader_type);
        let pipeline = Self::create_pbr_pipeline(&device, sc.format, &light_bind_group_layout, shader_type);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        let renderer = Renderer { queue, pipeline, depth_tex_view, light_bind_group, light_buffer };
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 200., 7., 3.);
        Self{ renderer, model, camera, time_in_flight: 0.0 }
    }

    fn render(&mut self, surface: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError> {
        self.camera.tick(0.01, &self.renderer.queue);
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
                            b: 0.2,
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
            render_pass.set_bind_group(3, &self.renderer.light_bind_group, &[]);
            render_pass.draw_model(&self.model, 2);
        }
        
        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
        self.camera.input(event);
        false
    }

    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
        self.time_in_flight += delta;
    }
}

impl GLTFViewerExample {
    fn create_output_pipeline(device: &wgpu::Device, tex_format: TextureFormat, light_bind_group_layout: &BindGroupLayout, /*shadow_tex_view: &TextureView, shadow_sampler: &Sampler,*/ shader_type: ShaderType) -> wgpu::RenderPipeline {
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
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
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                }]
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            label: Some("camera_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Output pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout, &device.create_bind_group_layout(&NOD_MM_BGL), &device.create_bind_group_layout(&MATERIAL_BGL),],
                push_constant_ranges: &[],
            }
        );
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
        })
    }

    fn create_pbr_pipeline(device: &wgpu::Device, tex_format: TextureFormat, light_bind_group_layout: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
        let buffer_layout = 
        [
            VertexBufferLayout{
                array_stride: std::mem::size_of::<[f32; 13]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                },
                VertexAttribute{
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: 4,
                }]
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
                    label: Some("PBR shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/pbr.wgsl").into()),
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
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            label: Some("camera_bind_group_layout"),
        });

        //0, 0 camera_params
        //0, 1 lighting_params
        //1, 0-10 textures
        //2, 0 node params
        
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Output pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout,
                                      &device.create_bind_group_layout(&MATERIAL_BGL),
                                      &device.create_bind_group_layout(&NOD_MM_BGL),
                                      light_bind_group_layout],
                push_constant_ranges: &[],
            }
        );
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

    fn get_light_matrix(time: f32) -> LightData {
        let distance = 10.;

        let light_position = glm::Vec3::new(time.sin() * distance, 10., time.cos() * distance);
        let light_view_matrix = glm::look_at(&light_position, &glm::Vec3::new(0.0, 0.0, 0.0), &glm::Vec3::new(0.0, 1.0, 0.0));
        let light_proj_matrix = glm::ortho(-0.5, 0.5, -0.5, 0.5, -15., 15.);
        
        let light_dir = -light_position.normalize();
        LightData {
            light_dir: [light_dir[0], light_dir[1], light_dir[2], 1.0],
            exposure: 5.1,
            gamma: 2.2,
            prefiltered_cube_mip_levels: 1.0,
            scale_IBL_Ambient: 0.2
        }
        // LightData {
        //     view_proj: (light_proj_matrix * light_view_matrix).into(),
        //     position: glm::Vec4::new(light_position.x, light_position.y, light_position.z, 0.0).into()
        // }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightData {
    light_dir:                      [f32; 4],
	exposure:                       f32,
	gamma:                          f32,
	prefiltered_cube_mip_levels:    f32,
	scale_IBL_Ambient:              f32,
}