use std::{iter, time::Duration, u32, num::NonZeroU32};
use image::GenericImageView;
use imgui::Context;
use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, Queue, ShaderModule, ShaderModuleDescriptor, ShaderStages, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupEntry, util::DeviceExt, BindGroupDescriptor, BindGroup, Buffer, VertexState, Device, BindGroupLayout, Surface, Texture};
use crate::{app::App, app::ShaderType, assets_helper::ResourceManager, input_event::{InputEvent, EventType}};

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    checker_bindgroup: BindGroup,
    cur_mip_bindgroup: BindGroup,
    cur_mip_buffer: Buffer,
    queue: Queue,

    imgui_context: Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

pub struct ImGUIExample {
    renderer : Renderer,
    resolution : Option<[u32; 2]>,
    selected_mip_level: usize,
    dropdown_items: Vec<String>,
}

impl<T: ResourceManager> App<T> for ImGUIExample{
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType,
        _: &T
    ) -> Self {
        let mut imgui_context = imgui::Context::create();
        imgui_context.io_mut().display_size = [sc.width as f32, sc.height as f32];
        let imgui_renderer = imgui_wgpu::Renderer::new(&mut imgui_context, &device, &queue, imgui_wgpu::RendererConfig{
            texture_format: sc.format,
            ..Default::default()
        });
        
        let checker_tex_bgl = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            label: Some("game_output_bind_group_layout"),
        });

        let cur_mip_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
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
        let tmp = [0.0; 4];
        let cur_mip_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cur mip buffer"),
            contents: bytemuck::cast_slice(&[tmp]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let cur_mip_bindgroup = device.create_bind_group(&BindGroupDescriptor{
            label: Some("Fullscreen triangle bindgroup"),
            layout: &cur_mip_bgl,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: cur_mip_buffer.as_entire_binding(),
            }],
        });
        
        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Full-screen triangle pipeline layout"),
                bind_group_layouts: &[&checker_tex_bgl, &cur_mip_bgl],
                push_constant_ranges: &[],
            }
        );
        let pipeline = ImGUIExample::create_render_pipeline(device, &pipeline_layout, sc.format, shader_type);
        
        let mut init_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let (texture, tex_bg, num_mips) = Self::create_texture(device, &queue, &checker_tex_bgl, sc.format);
        Self::generate_mipmaps(
            &mut init_encoder,
            device,
            &texture,
            num_mips,
        );
        queue.submit(Some(init_encoder.finish()));

        let dropdown_items: Vec<String> = (0..num_mips).map(|i|{
            format!("Mip level {}", i)
        }).collect();

        let renderer = Renderer {
            pipeline: pipeline,
            checker_bindgroup: tex_bg,
            cur_mip_bindgroup,
            cur_mip_buffer,
            queue,
            imgui_context,
            imgui_renderer,
        };

        Self{renderer, resolution: None, selected_mip_level: 0, dropdown_items}
    }

    fn resize(&mut self, sc: &wgpu::SurfaceConfiguration, _device: &wgpu::Device) {
        self.resolution = Some([sc.width, sc.height]);
    }

    fn tick(&mut self, delta: f32) {
        self.renderer.imgui_context.io_mut().update_delta_time(Duration::from_secs_f32(delta));
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
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
            let cur_mip = [self.selected_mip_level as f32, 0.0, 0.0, 0.0];
            self.renderer.queue.write_buffer(&self.renderer.cur_mip_buffer, 0, bytemuck::cast_slice(&[cur_mip]));

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
            render_pass.set_bind_group(0, &self.renderer.checker_bindgroup, &[]);
            render_pass.set_bind_group(1, &self.renderer.cur_mip_bindgroup, &[]);
            render_pass.draw(0..3, 0..1);
        
            let ui = self.renderer.imgui_context.frame();
            ui.window("Settings")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    if let Some(_) = ui.begin_combo("Mip level", self.dropdown_items[self.selected_mip_level].as_str()) {
                        for (index, val) in self.dropdown_items.iter().enumerate() {
                            if self.selected_mip_level == index {
                                ui.set_item_default_focus();
                            }
                            let clicked = ui.selectable_config(val)
                                .selected(self.selected_mip_level == index)
                                .build();
                            if clicked {
                                self.selected_mip_level = index;
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

impl ImGUIExample {
    fn create_render_pipeline(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        tex_format: TextureFormat,
        shader_type: ShaderType
    ) -> wgpu::RenderPipeline {
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
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/fullscreen_tri.wgsl").into()),
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
            _ => panic!("No spirv shaders found")
        }

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SimpleTriApp pipeline"),
            layout: Some(pipeline_layout),
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

    fn create_texture(device: &Device, queue: &Queue, bgl: &BindGroupLayout, frmt: TextureFormat) -> (Texture, BindGroup, u32) {
        let checker_image = image::load_from_memory(include_bytes!("../../../assets/textures/checker.png")).unwrap();
        let checker_rgba = checker_image.to_rgba8();
        let (checker_width, checker_height) = checker_image.dimensions();

        let num_mips = (f32::log2(checker_width.min(checker_height) as f32).floor() + 1.0) as u32;

        let checker_size = wgpu::Extent3d {
            width: checker_width,
            height: checker_height,
            depth_or_array_layers: 1,
        };
        let checker_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: checker_size,
                mip_level_count: num_mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING |
                       wgpu::TextureUsages::COPY_DST |
                       wgpu::TextureUsages::RENDER_ATTACHMENT,
                label: Some("Checker texture"),
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &checker_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &checker_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * checker_width),
                rows_per_image: std::num::NonZeroU32::new(checker_height),
            },
            checker_size,
        );

        let checker_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let checker_texture_view = checker_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bg = device.create_bind_group(&BindGroupDescriptor{
            label: Some("BG tv 0"),
            layout: bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&checker_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&checker_sampler),
                }
            ],
        });
        (checker_texture, bg, num_mips)
    }

    fn generate_mipmaps(
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        mip_count: u32,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wgsl/blit.wgsl").into()),
        });
        const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(TEXTURE_FORMAT.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let views = (0..mip_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mip"),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect::<Vec<_>>();

        for target_mip in 1..mip_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
    }
}