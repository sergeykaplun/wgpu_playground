use std::{iter, mem};
use std::f32::consts::PI;
use std::time::Duration;
use image::GenericImageView;
use imgui::Context;
use wgpu::{Queue, TextureFormat, VertexBufferLayout, VertexAttribute, ColorTargetState, VertexState, FragmentState, ShaderModule, PrimitiveState, Face, DepthStencilState, StencilState, DepthBiasState, MultisampleState, ShaderModuleDescriptor, RenderPipeline, RenderPassDepthStencilAttachment, Operations, TextureView, BindGroup, Buffer, BindGroupLayout, BindingResource, Device, Sampler, include_spirv_raw, Features};
use wgpu::Face::Back;
use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}, model::{GLTFModel, Drawable, NOD_MM_BGL, MATERIAL_BGL, parse_gltf}, assets_helper::ResourceManager, input_event::InputEvent, skybox::{Skybox, DrawableSkybox}};
use crate::app::AppVariant;
use crate::input_event::EventType;

const DEBUG_TEX_ITEMS: [&str; 7] = ["none", "base color", "normal", "occlusion", "emissive", "metallic", "roughness"];
const DEBUG_ITEMS: [&str; 6] = ["None", "diff(l, n)", "F(l,h)", "G(l,v,h)", "D(h)", "Specular"];

struct Renderer {
    queue: Queue,
    
    pipeline: RenderPipeline,
    depth_tex_view: TextureView,

    light_buffer: Buffer,
    light_bind_group: BindGroup,

    imgui_context: Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

pub struct PBRExample {
    renderer: Renderer,
    model: GLTFModel,
    skybox: Skybox,
    camera: ArcballCamera,
    time_in_flight: f32,
    debug_view_texture: usize,
    debug_view_item: usize,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

impl<T: ResourceManager> App<T> for PBRExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: ShaderType,
        resource_manager: &T
    ) -> Self {
        let mut imgui_context = imgui::Context::create();
        imgui_context.io_mut().display_size = [sc.width as f32, sc.height as f32];
        let imgui_renderer = imgui_wgpu::Renderer::new(&mut imgui_context, &device, &queue, imgui_wgpu::RendererConfig{
            texture_format: sc.format,
            depth_format: Some(TextureFormat::Depth24Plus),
            ..Default::default()
        });

        //let model = pollster::block_on(parse_gltf("models/DamagedHelmet/glTF-Embedded/DamagedHelmet.gltf", &device, &queue, resource_manager));
        let model = pollster::block_on(parse_gltf("./models/maserati_ghibli_hybrid/scene.gltf", &device, &queue, resource_manager));
        //let model = pollster::block_on(parse_gltf("./models/vehicle_zis-101/scene.gltf", &device, &queue, resource_manager));
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
        let skybox = Skybox::default_ktx(device, &queue, sc.format, shader_type, &camera_bind_group_layout);
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
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(light_uniform_size),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::Cube,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::Cube,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                }
            );
            let brdf_lut = Self::brdf_lut_texture(device, &queue);
            let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &light_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: light_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&skybox.irradiance_tv),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&skybox.irradiance_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&skybox.prefiltered_envmap_tv),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&skybox.prefiltered_envmap_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(&brdf_lut.0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: BindingResource::Sampler(&brdf_lut.1),
                    },
                ],
                label: None,
            });
            (light_bind_group_layout, light_bind_group, light_buf)
        };

        //let pipeline = Self::create_pbr_pipeline(&device, sc.format, &light_bind_group_layout, &camera_bind_group_layout, shader_type);
        let pipeline = Self::create_pbr_pipeline(&device, sc.format, &light_bind_group_layout, &camera_bind_group_layout, ShaderType::SPIRV);
        let depth_tex_view = Self::create_depth_texture(sc, device);
        let renderer = Renderer { queue, pipeline, depth_tex_view, light_bind_group, light_buffer, imgui_context, imgui_renderer };
        let mut camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 200., 7., 6.);
        camera.azimuth = PI / 4.;
        camera.polar = -PI / 4.;
        Self{ renderer, model, skybox, camera, time_in_flight: 0.0, debug_view_texture: 0, debug_view_item: 0 }
    }

    fn get_extra_device_features(_app_variant: AppVariant) -> Features {
        Features::SPIRV_SHADER_PASSTHROUGH
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
        self.camera.input(event);
        match event.event_type {
            EventType::Move => self.renderer.imgui_context.io_mut().mouse_pos = [event.coords[0] as f32, event.coords[1] as f32],
            EventType::Start => {
                self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = true;
            },
            EventType::End => self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = false,
            EventType::None => (),
        };
        false
    }

    fn tick(&mut self, delta: f32) {
        self.camera.tick(delta, &self.renderer.queue);
        self.renderer.imgui_context.io_mut().update_delta_time(Duration::from_secs_f32(delta));
        self.time_in_flight += delta;
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

            //render_pass.draw_skybox(&self.skybox, &self.camera.camera_bind_group);

            let ui = self.renderer.imgui_context.frame();
            ui.window("Settings")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    if let Some(_) = ui.begin_combo("Debug texture", DEBUG_TEX_ITEMS[self.debug_view_texture]) {
                        for (index, val) in DEBUG_TEX_ITEMS.iter().enumerate() {
                            if self.debug_view_texture == index {
                                ui.set_item_default_focus();
                            }
                            let clicked = ui.selectable_config(val)
                                .selected(self.debug_view_texture == index)
                                .build();
                            if clicked {
                                self.debug_view_texture = index;
                            }
                        }
                    }
                    if let Some(_) = ui.begin_combo("Debug view", DEBUG_ITEMS[self.debug_view_item]) {
                        for (index, val) in DEBUG_ITEMS.iter().enumerate() {
                            if self.debug_view_item == index {
                                ui.set_item_default_focus();
                            }
                            let clicked = ui.selectable_config(val)
                                .selected(self.debug_view_item == index)
                                .build();
                            if clicked {
                                self.debug_view_item = index;
                            }
                        }
                    }
                });
            let draw_data = self.renderer.imgui_context.render();
            self.renderer.imgui_renderer.render(draw_data, &self.renderer.queue, device, &mut render_pass).unwrap();
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl PBRExample {
    fn create_pbr_pipeline(device: &wgpu::Device, tex_format: TextureFormat, light_bind_group_layout: &BindGroupLayout, camera_bind_group_layout: &BindGroupLayout, shader_type: ShaderType) -> wgpu::RenderPipeline {
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
            ShaderType::SPIRV => {
                unsafe {
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/pbr.vert.spv")));
                    spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/pbr.frag.spv")));
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
            }
        }
        
        //0, 0 camera_params
        //1, 0-10 textures
        //2, 0 node params
        //3, 0 lighting_params
        
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
                cull_mode: None,
                //cull_mode: Some(Back),
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

    fn brdf_lut_texture(device: &Device, queue: &Queue) -> (TextureView, Sampler){
        let brdf_lut_image = image::load_from_memory(include_bytes!("../../../assets/textures/brdf_lut.png")).unwrap();
        let brdf_lut_rgba = brdf_lut_image.to_rgba8();
        let (brdf_lut_width, brdf_lut_height) = brdf_lut_image.dimensions();

        let brdf_lut_size = wgpu::Extent3d {
            width: brdf_lut_width,
            height: brdf_lut_height,
            depth_or_array_layers: 1,
        };
        let brdf_lut_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: brdf_lut_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("brdf lut texture"),
                view_formats: &[],
            }
        );
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &brdf_lut_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &brdf_lut_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * brdf_lut_width),
                rows_per_image: std::num::NonZeroU32::new(brdf_lut_height),
            },
            brdf_lut_size,
        );

        let brdf_lut_texture_view = brdf_lut_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let brdf_lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        (brdf_lut_texture_view, brdf_lut_sampler)
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