use std::{iter, time::Duration};
use imgui::Context;
use wgpu::{PrimitiveState, Face, MultisampleState, FragmentState, ColorTargetState, TextureFormat, Queue, ShaderModule, ShaderModuleDescriptor, ShaderStages, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindGroupEntry, util::DeviceExt, BindGroupDescriptor, BindGroup, Buffer, VertexState};
use crate::{app::App, app::ShaderType, assets_helper::ResourceManager, input_event::{InputEvent, EventType}};

const DROP_DOWN_ITEMS: [&str; 2] = ["Item 1", "Item 2"];
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: Buffer,
    uniform_bindgroup: BindGroup,
    queue: Queue,

    imgui_context: Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

pub struct ImGUIExample {
    renderer : Renderer,
    resolution : Option<[u32; 2]>,
    selected_mip_level: usize,
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
            pipeline: ImGUIExample::create_render_pipeline(device, &pipeline_layout, sc.format, shader_type),
            uniform_buffer,
            uniform_bindgroup,
            queue,
            imgui_context,
            imgui_renderer,
            //imgui_tv
        };

        Self{renderer, resolution: None, selected_mip_level: 0}
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
        
            let ui = self.renderer.imgui_context.frame();
            ui.window("Settings")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    if let Some(_) = ui.begin_combo("Mip level", DROP_DOWN_ITEMS[self.selected_mip_level]) {
                        for (index, val) in DROP_DOWN_ITEMS.iter().enumerate() {
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
}