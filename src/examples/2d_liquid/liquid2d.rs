use std::iter;
use std::convert::Into;
use std::default::Default;
use std::mem::size_of;
use std::time::Duration;
use glm::sqrt;
use imgui::Context;
use wgpu::{Adapter, BindGroupLayout, BlendState, BufferAddress, ColorTargetState, ComputePassDescriptor, ComputePipeline, Device, FragmentState, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor, ShaderSource, Surface, SurfaceConfiguration, SurfaceError, TextureFormat, VertexState};
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
    predicted_pos: [f32; 2],
    velocity: [f32; 2],
    density: f32,
    _padding: f32,
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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
    pointer_location: [f32; 2],

    resolution: [f32; 2],
    pointer_active: f32,
    pointer_attract: f32,

    group_width: u32,
    group_height: u32,
    step_index: u32,
    _padding2: u32,
}

struct Renderer {
    queue: Queue,
    constants_buffer: wgpu::Buffer,
    constants_bg: wgpu::BindGroup,
    particles_buffer: wgpu::Buffer,
    particle_read_bg: wgpu::BindGroup,
    draw_particles_pso: RenderPipeline,

    particle_read_write_bg: wgpu::BindGroup,
    overlay_pso: RenderPipeline,
    particles_pre_update_pso: ComputePipeline,
    compute_particle_densities_pso: ComputePipeline,
    apply_particles_pressure_pso: ComputePipeline,
    update_particles_positions_pso: ComputePipeline,

    spatial_lookup_bg: wgpu::BindGroup,
    compute_spatial_lookup_cp: ComputePipeline,
    write_start_indices_cp: ComputePipeline,
    sort_lookup_cp: ComputePipeline,

    imgui_context: Context,
    imgui_renderer: imgui_wgpu::Renderer,
}

pub struct Liquid2DExample {
    renderer: Renderer,
    constants: Constants,
}

impl Liquid2DExample {
    const SOLVER_FPS: f32 = 30f32;
    const SOLVER_DELTA_TIME: f32 = 1f32 / Self::SOLVER_FPS;
    const PARTICLES_CNT: usize = 4096;
    const WORKGROUP_SIZE: usize = 256;
    const WORKGROUP_CNT: u32 = ((Self::PARTICLES_CNT + Self::WORKGROUP_SIZE - 1) / Self::WORKGROUP_SIZE) as u32;
    const SIM_BOUNDS: [f32; 2] = [16., 9.];
    const PARTICLE_RADIUS: f32 = 0.05;
    const DEFAULT_GRAVITY: [f32; 2] = [0.0, -9.8];
    const DEFAULT_PARTICLE_MASS: f32 = 1.;
    const DEFAULT_PARTICLE_SEGMENTS: u32 = 24;
    const DEFAULT_DAMPING: f32 = 0.95;
    const DEFAULT_SMOOTHING_RADIUS: f32 = 0.3;
    const DEFAULT_TARGET_DENSITY: f32 = 1.5;
    const DEFAULT_PRESSURE_MULTIPLIER: f32 = 1.1;
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

        let particles_per_row = (Self::PARTICLES_CNT as f32).sqrt().floor() as i32;
        let particles_per_col = (Self::PARTICLES_CNT - 1) as i32 / particles_per_row + 1i32;
        let particle_spacing = 1.;
        let spacing = Self::PARTICLE_RADIUS * 2.0 * particle_spacing;
        let buffer: [Particle; Self::PARTICLES_CNT] = (0..Self::PARTICLES_CNT).map(|i|{
            let x = ((((i as i32) % particles_per_row - particles_per_row / 2) as f32) + 0.5) * spacing;
            let y = ((((i as i32) / particles_per_row - particles_per_col / 2) as f32) + 0.5) * spacing;
            //let x = rand::random::<f32>() * Self::SIM_BOUNDS[0] - Self::SIM_BOUNDS[0] / 2.;
            //let y = rand::random::<f32>() * Self::SIM_BOUNDS[1] - Self::SIM_BOUNDS[1] / 2.;
            Particle{
                position: [x, y],
                velocity: [0.0; 2],
                density: 0.0,
                predicted_pos: [0.0; 2],
                _padding: 0.0,
            }
        }).collect::<Vec<_>>().try_into().unwrap();

        let particles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle Buffer"),
            contents: bytemuck::cast_slice(&buffer),
            usage: wgpu::BufferUsages::STORAGE
        });
        let spatial_lookup_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("Spetial Lookup Buffer"),
            size: (Self::PARTICLES_CNT * size_of::<u32>() * 2) as BufferAddress,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let start_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("Start Indices Buffer"),
            size: (Self::PARTICLES_CNT * size_of::<u32>()) as BufferAddress,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
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
            entries: &[
                wgpu::BindGroupLayoutEntry{
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
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });
        let particle_read_write_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Particle Data Write Bind Group"),
            layout: &particle_data_write_bgl,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: particles_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: spatial_lookup_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: start_indices_buffer.as_entire_binding(),
                },
            ],
        });


        let spatial_lookup_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Spatial Lookup Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });
        let spatial_lookup_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Spatial Lookup Bind Group"),
            layout: &spatial_lookup_bgl,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: particles_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: spatial_lookup_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: start_indices_buffer.as_entire_binding(),
                }
            ],
        });

        let constants = Constants{
            gravity: Self::DEFAULT_GRAVITY,
            smoothing_radius: Self::DEFAULT_SMOOTHING_RADIUS,
            particle_mass: Self::DEFAULT_PARTICLE_MASS,
            particle_segments: Self::DEFAULT_PARTICLE_SEGMENTS,
            aspect: sc.width as f32 / sc.height as f32,
            particle_radius: Self::PARTICLE_RADIUS,
            bounds_size: Self::SIM_BOUNDS,
            damping: Self::DEFAULT_DAMPING,
            particles_count: Self::PARTICLES_CNT as u32,
            target_density: Self::DEFAULT_TARGET_DENSITY,
            pressure_multiplier: Self::DEFAULT_PRESSURE_MULTIPLIER,
            resolution: [sc.width as f32, sc.height as f32],
            ..Default::default()
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

        //let process_particles_pso = Self::create_process_particles_pso(device, &constants_bgl, &particle_data_write_bgl);
        let (particles_pre_update_pso, compute_particle_densities_pso, apply_particles_pressure_pso, update_particles_positions_pso) = Self::create_particles_update_psos(device, &constants_bgl, &particle_data_write_bgl);
        let draw_particles_pso = Self::create_draw_particles_pso(device, sc.format, shader_type, &constants_bgl, &particle_data_read_bgl);
        let overlay_pso = Self::create_overlay_pso(device, sc.format, &constants_bgl, &particle_data_read_bgl);
        let compute_spatial_lookup_cp = Self::create_spatial_lookup_pso(device, &constants_bgl, &spatial_lookup_bgl);
        let write_start_indices_cp = Self::create_start_indices_pso(device, &constants_bgl, &spatial_lookup_bgl);
        let sort_lookup_cp = Self::create_sort_lookup_pso(device, &constants_bgl, &spatial_lookup_bgl);

        let renderer = Renderer{ queue, constants_buffer, constants_bg, particles_buffer,
                                 particle_read_bg, draw_particles_pso, particle_read_write_bg, overlay_pso,
                                 particles_pre_update_pso, compute_particle_densities_pso, apply_particles_pressure_pso, update_particles_positions_pso,
                                 spatial_lookup_bg, compute_spatial_lookup_cp, write_start_indices_cp, sort_lookup_cp,
                                 imgui_context, imgui_renderer };
        Liquid2DExample{ renderer, constants }
    }

    fn tick(&mut self, device: &Device, delta: f32) {
        self.constants.delta_time += delta;
        self.renderer.imgui_context.io_mut().update_delta_time(Duration::from_secs_f32(delta));

        if self.constants.delta_time > Self::SOLVER_DELTA_TIME {
            self.solver_step(device);
        }
        //if self.constants.delta_time > 0.05 {
        //    self.constants.delta_time = 0.05;
        //    self.solver_step(device);
        //}
        //self.constants.time += delta;
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
        match event.event_type {
            EventType::Move => {
                self.renderer.imgui_context.io_mut().mouse_pos = [event.coords[0], event.coords[1]];
                self.constants.pointer_location = event.coords;
            },
            EventType::Start(btn) => {
                self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = true;
                self.constants.pointer_active = 1.0;
                self.constants.pointer_attract = btn as f32;
            },
            EventType::End => {
                self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = false;
                self.constants.pointer_active = 0.0;
            },
            EventType::None => (),
        };
        false
    }

    fn render(&mut self, frame: &Surface, device: &Device) -> Result<(), SurfaceError> {
        //self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));
        //self.solver_step(device);

        let output = frame.get_current_texture()?;
        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Draw particles"),
            });
        {
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
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
                    ui.slider("Smoothing rad", 0.01, 3., &mut self.constants.smoothing_radius);
                    ui.slider("Particle mass", 0.01, 10., &mut self.constants.particle_mass);
                    ui.slider("Target density", 0., 10., &mut self.constants.target_density);
                    ui.slider("Pressure multiplier", 0., 30., &mut self.constants.pressure_multiplier);
                });
            let draw_data = self.renderer.imgui_context.render();
            self.renderer.imgui_renderer.render(draw_data, &self.renderer.queue, device, &mut render_pass).unwrap();
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
        output.present();

        //self.constants.delta_time = 0f32;
        Ok(())
    }
}

impl Liquid2DExample {
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

    /*fn create_process_particles_pso(device: &Device, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> ComputePipeline {
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
    }*/

    fn create_particles_update_psos(device: &Device, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> (ComputePipeline, ComputePipeline, ComputePipeline, ComputePipeline) {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Process particles shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/process.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Process Particles Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        //particles_pre_update_pso, compute_particle_densities_pso, apply_particles_pressure_pso, update_particles_positions_pso

        (
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Particles PreUpdate Pipeline"),
                layout: Some(&rpl),
                module: &shader_module,
                entry_point: "calculate_predicted_pos",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Calculate Particle Densities Pipeline"),
                layout: Some(&rpl),
                module: &shader_module,
                entry_point: "calculate_particle_densities",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Apply Particles Pressure Pipeline"),
                layout: Some(&rpl),
                module: &shader_module,
                entry_point: "apply_particles_pressure",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Update Particles Positions Pipeline"),
                layout: Some(&rpl),
                module: &shader_module,
                entry_point: "update_particles_positions",
            })
        )
    }

    fn create_spatial_lookup_pso(device: &Device, constants_bgl: &BindGroupLayout, spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Create spatial lookup shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/spatial_lookup.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Create spatial lookup pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Create spatial lookup Pipeline"),
            layout: Some(&rpl),
            module: &shader_module,
            entry_point: "write_spatial_lookup",
        })
    }

    fn create_start_indices_pso(device: &Device, constants_bgl: &BindGroupLayout, spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Write start indices shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/spatial_lookup.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Write start indices pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("write start indices Pipeline"),
            layout: Some(&rpl),
            module: &shader_module,
            entry_point: "write_start_indices",
        })
    }

    fn create_sort_lookup_pso(device: &Device, constants_bgl: &BindGroupLayout, spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sort lookup shader module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/spatial_lookup.wgsl").into()),
            //source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/bitonic_sort.wgsl").into()),
        });

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sort lookup pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Sort lookup pipeline"),
            layout: Some(&rpl),
            module: &shader_module,
            entry_point: "sort_pairs",
            //entry_point: "sort",
            //entry_point: "bitonic_sort_pairs",
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

    //sorry
    fn next_power_of_two(n: u32) -> u32 {
        let mut result = 1;
        while result < n {
            result <<= 1;
        }
        result
    }

    fn solver_step(&mut self, device: &Device) {
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));
        self.update_particles_velocities(device);
        //self.update_spatial_lookup(device);
        self.calculate_particle_densities(device);
        self.apply_particle_pressure(device);
        self.update_particles_positions(device);

        /*let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Write spatial lookup encoder"),
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

        self.renderer.queue.submit(iter::once(encoder.finish()));*/
        self.constants.delta_time = 0f32;
    }

    fn update_particles_velocities(&mut self, device: &Device) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Update Particle Velocities Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.particles_pre_update_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        self.renderer.queue.submit(iter::once(encoder.finish()));
    }

    fn calculate_particle_densities(&mut self, device: &Device) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Calculate Particle Densities Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.compute_particle_densities_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        self.renderer.queue.submit(iter::once(encoder.finish()));
    }

    fn apply_particle_pressure(&mut self, device: &Device) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Apply Particle pressure Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.apply_particles_pressure_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        self.renderer.queue.submit(iter::once(encoder.finish()));
    }

    fn update_particles_positions(&mut self, device: &Device) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Update Particle positions Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.update_particles_positions_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        self.renderer.queue.submit(iter::once(encoder.finish()));
    }

    fn update_spatial_lookup(&mut self, device: &Device) {
        {
            let mut encoder = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Write spatial lookup encoder"),
                });
            {
                let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                    label: Some("Write spatial lookup compute pass"),
                });
                cp.set_pipeline(&self.renderer.compute_spatial_lookup_cp);
                cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
                cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
                cp.dispatch_workgroups(Self::PARTICLES_CNT as u32, 1, 1);
            }
            self.renderer.queue.submit(iter::once(encoder.finish()));
        }
        self.bitonic_sort(device);
        {
            let mut encoder = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Process particles encoder"),
                });
            {
                let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                    label: Some("Write start indices compute pass"),
                });
                cp.set_pipeline(&self.renderer.write_start_indices_cp);
                cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
                cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
                cp.dispatch_workgroups(Self::PARTICLES_CNT as u32, 1, 1);
            }
            self.renderer.queue.submit(iter::once(encoder.finish()));
        }
    }

    fn bitonic_sort(&mut self, device: &Device) {
        let num_pairs = Self::next_power_of_two(Self::PARTICLES_CNT as u32) / 2;
        let num_stages = ((num_pairs * 2) as f32).log(2.0) as u32;

        for stage_index in 0..num_stages {
            for step_index in 0..(stage_index + 1) {
                let group_width = 1 << (stage_index - step_index);
                let group_height = 2 * group_width - 1;
                self.constants.group_width = group_width;
                self.constants.group_height = group_height;
                self.constants.step_index = step_index;
                self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));

                let mut encoder = device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Bitonic sort encoder"),
                    });
                {
                    let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("Bitonic step compute pass"),
                    });
                    cp.set_pipeline(&self.renderer.sort_lookup_cp);
                    cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
                    cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
                    //cp.dispatch_workgroups(num_pairs, 1, 1);
                    //cp.dispatch_workgroups(Self::PARTICLES_CNT as u32 / 128, 1, 1);
                    cp.dispatch_workgroups(num_pairs as u32 / 128, 1, 1);
                }
                self.renderer.queue.submit(iter::once(encoder.finish()));
            }
        }
    }

    fn local_bitonic_merge_sort(&mut self, h: u32, device: &Device, workgroup_count: u32) {
        self.constants.group_height = h;
        self.constants.step_index = 0;
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LOCAL_BITONIC_MERGE_SORT"),
            });
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("LOCAL_BITONIC_MERGE_SORT compute pass"),
            });
            cp.set_pipeline(&self.renderer.sort_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(workgroup_count, 1, 1);
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
    }
    fn big_flip(&mut self, h: u32, device: &Device, workgroup_count: u32) {
        self.constants.group_height = h;
        self.constants.step_index = 2;
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BIG_FLIP"),
            });
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("BIG_FLIP compute pass"),
            });
            cp.set_pipeline(&self.renderer.sort_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(workgroup_count, 1, 1);
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
    }
    fn local_disperse(&mut self, h: u32, device: &Device, workgroup_count: u32) {
        self.constants.group_height = h;
        self.constants.step_index = 1;
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LOCAL_DISPERSE"),
            });
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("LOCAL_DISPERSE compute pass"),
            });
            cp.set_pipeline(&self.renderer.sort_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(workgroup_count, 1, 1);
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
    }
    fn big_disperse(&mut self, h: u32, device: &Device, workgroup_count: u32) {
        self.constants.group_height = h;
        self.constants.step_index = 3;
        self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BIG_DISPERSE"),
            });
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("BIG_DISPERSE compute pass"),
            });
            cp.set_pipeline(&self.renderer.sort_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(workgroup_count, 1, 1);
        }

        self.renderer.queue.submit(iter::once(encoder.finish()));
    }
}