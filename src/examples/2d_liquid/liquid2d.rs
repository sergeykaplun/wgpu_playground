use std::iter;
use std::time::Duration;
use imgui::Context;
use rand::random;
use wgpu::{Adapter, BindGroupLayout, BlendState, ColorTargetState, ComputePassDescriptor, ComputePipeline, Device, FragmentState, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderSource, Surface, SurfaceConfiguration, SurfaceError, TextureFormat, VertexState};
use wgpu::BufferBindingType::{Storage, Uniform};
use wgpu::Face::Back;
use wgpu::FrontFace::Ccw;
use wgpu::PolygonMode::Fill;
use wgpu::PrimitiveTopology::TriangleList;
use wgpu::util::DeviceExt;
use crate::app::{App, ShaderType};
use crate::assets_helper::ResourceManager;
use crate::input_event::{EventType, InputEvent};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
    density: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    gravity: [f32; 2],
    smoothing_radius: f32,
    particle_mass: f32,

    aspect: f32,
    particle_segments: u32,
    particle_radius: f32,
    delta_time: f32,

    bounds_size: [f32; 2],
    damping: f32,
    particles_count: u32,

    target_density: f32,// = 20.75;
    pressure_multiplier: f32,// = 0.5;
    _padding: [f32; 2],
}

struct Renderer {
    queue: Queue,
    constants_buffer: wgpu::Buffer,
    constants_bg: wgpu::BindGroup,
    particles_buffer: wgpu::Buffer,
    particle_read_bg: wgpu::BindGroup,
    draw_particles_pso: RenderPipeline,

    particle_read_write_bg: wgpu::BindGroup,
    process_particles_pso: ComputePipeline,
    overlay_pso: RenderPipeline,

    imgui_context: Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

pub struct Liquid2DExample {
    renderer: Renderer,
    constants: Constants,
}

impl<T: ResourceManager> App<T> for Liquid2DExample {
    fn new(sc: &SurfaceConfiguration, adapter: &Adapter, device: &Device, queue: Queue, shader_type: ShaderType, resource_manager: &T) -> Self {
        let mut imgui_context = imgui::Context::create();
        imgui_context.io_mut().display_size = [sc.width as f32, sc.height as f32];
        let imgui_renderer = imgui_wgpu::Renderer::new(&mut imgui_context, &device, &queue, imgui_wgpu::RendererConfig{
            texture_format: sc.format,
            depth_format: None,
            ..Default::default()
        });

        let buffer: [Particle; Self::PARTICLES_CNT] = (0..Self::PARTICLES_CNT).map(|_|{
            Particle{
                position: [(random::<f32>() * 2.0 - 1.0) * Self::SIM_BOUNDS[0] * 0.5, (random::<f32>() * 2.0 - 1.0) * Self::SIM_BOUNDS[1] * 0.5],
                //position: [(random::<f32>() * 2.0 - 1.0) * Self::SIM_BOUNDS[0] * 0.5, 0.0],
                velocity: [0.0, 0.0],
                density: 0.0,
                _padding: 0.0,
            }
        }).collect::<Vec<_>>().try_into().unwrap();
        let particles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&buffer),
            usage: wgpu::BufferUsages::STORAGE
        });
        let particle_data_read_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Particle Data Read Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: Storage {
                        read_only: true,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let particle_read_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Particle Data Read Bind Group"),
            layout: &particle_data_read_bgl,
            entries: &[wgpu::BindGroupEntry{
                binding: 0,
                resource: particles_buffer.as_entire_binding(),
            }],
        });

        let particle_data_write_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Particle Data Write Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: Storage {
                        read_only: false,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let particle_read_write_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Particle Data Write Bind Group"),
            layout: &particle_data_write_bgl,
            entries: &[wgpu::BindGroupEntry{
                binding: 0,
                resource: particles_buffer.as_entire_binding(),
            }],
        });

        let constants = Constants{
            gravity: [0.0, -9.8],
            smoothing_radius: 1.2,  //0.35
            particle_mass: 1.,
            particle_segments: 24,
            aspect: sc.width as f32 / sc.height as f32,
            delta_time: 0.0,
            particle_radius: 0.1, //0.0125,
            bounds_size: Self::SIM_BOUNDS,
            damping: 0.95,
            particles_count: Self::PARTICLES_CNT as u32,
            target_density: 2.75, //20.75,
            pressure_multiplier: 0.5,
            _padding: [0.0; 2],
        };
        let constants_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Constants Buffer"),
            contents: bytemuck::cast_slice(&[constants]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let constants_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Constants Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let constants_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Constants Bind Group"),
            layout: &constants_bgl,
            entries: &[wgpu::BindGroupEntry{
                binding: 0,
                resource: constants_buffer.as_entire_binding(),
            }],
        });

        let process_particles_pso = Self::create_process_particles_pso(device, &constants_bgl, &particle_data_write_bgl);
        let draw_particles_pso = Self::create_draw_particles_pso(device, sc.format, shader_type, &constants_bgl, &particle_data_read_bgl);
        let overlay_pso = Self::create_overlay_pso(device, sc.format, &constants_bgl, &particle_data_read_bgl);

        let renderer = Renderer{ queue, constants_buffer, constants_bg, particles_buffer, particle_read_bg, draw_particles_pso, particle_read_write_bg, process_particles_pso, overlay_pso, imgui_context, imgui_renderer };
        Liquid2DExample{ renderer, constants }
    }

    fn tick(&mut self, delta: f32) {
        self.constants.delta_time += delta;
        self.renderer.imgui_context.io_mut().update_delta_time(Duration::from_secs_f32(delta));
        //self.constants.time += delta;
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

    fn render(&mut self, frame: &Surface, device: &Device) -> Result<(), SurfaceError> {
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));
        self.constants.delta_time = 0f32;
        let output = frame.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Process Particles Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.process_particles_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::PARTICLES_CNT as u32, 1, 1);
        }

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
                depth_stencil_attachment: None
            });

            render_pass.set_pipeline(&self.renderer.overlay_pso);
            render_pass.set_bind_group(0, &self.renderer.constants_bg, &[]);
            render_pass.set_bind_group(1, &self.renderer.particle_read_bg, &[]);
            render_pass.draw(0..3, 0..1);

            render_pass.set_pipeline(&self.renderer.draw_particles_pso);
            render_pass.set_bind_group(0, &self.renderer.constants_bg, &[]);
            render_pass.set_bind_group(1, &self.renderer.particle_read_bg, &[]);
            render_pass.draw(0..Self::PARTICLES_CNT as u32 * 3 * self.constants.particle_segments, 0..1);

            let ui = self.renderer.imgui_context.frame();
            ui.window("Settings")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.slider("Smoothing rad", 0., 3., &mut self.constants.smoothing_radius);
                    ui.slider("Particle mass", 0., 10., &mut self.constants.particle_mass);
                    ui.slider("Target density", 0., 100., &mut self.constants.target_density);
                    ui.slider("Pressure multiplier", 0., 500., &mut self.constants.pressure_multiplier);
                });
            let draw_data = self.renderer.imgui_context.render();
            self.renderer.imgui_renderer.render(draw_data, &self.renderer.queue, device, &mut render_pass).unwrap();
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl Liquid2DExample {
    const PARTICLES_CNT: usize = 400;
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
    const SIM_BOUNDS: [f32; 2] = [16., 9.];
    //const SIM_BOUNDS: [f32; 2] = [2.; 2];

    fn create_draw_particles_pso(device: &Device, tex_format: TextureFormat, shader_type: ShaderType, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Visualize particle shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/visualize.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Visualize Particle Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor{
            label: Some("Draw Particles Pipeline"),
            layout: Some(&rpl),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: PrimitiveState{
                topology: TriangleList,
                strip_index_format: None,
                front_face: Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState{
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState{
                    format: tex_format,
                    blend: Default::default(),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
        })
    }

    fn create_process_particles_pso(device: &Device, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> ComputePipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Process particles shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/process.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Process Particles Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Process Particles Pipeline"),
            layout: Some(&rpl),
            module: &shader_module,
            entry_point: "process_particles",
        })
    }

    fn create_overlay_pso(device: &Device, tex_format: TextureFormat, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> RenderPipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Process particles shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/overlay.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Process Particles Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        let transparent_blend_state = BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        device.create_render_pipeline(&RenderPipelineDescriptor{
            label: Some("Overlay Pipeline"),
            layout: Some(&rpl),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: PrimitiveState{
                topology: TriangleList,
                strip_index_format: None,
                front_face: Ccw,
                cull_mode: Some(Back),
                unclipped_depth: false,
                polygon_mode: Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState{
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState{
                    format: tex_format,
                    blend: Some(transparent_blend_state),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
        })
    }
}