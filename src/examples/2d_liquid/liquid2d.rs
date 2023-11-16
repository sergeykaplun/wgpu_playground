use std::iter;
use std::default::Default;
use std::mem::size_of;
use std::time::Duration;
use imgui::Context;
use wgpu::{Adapter, BindGroupLayout, BlendState, BufferAddress, ColorTargetState, ComputePassDescriptor, ComputePipeline, Device, Features, FragmentState, include_spirv_raw, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, Queue, RenderPipeline, RenderPipelineDescriptor, Sampler, ShaderModule, ShaderStages, Surface, SurfaceConfiguration, SurfaceError, TextureFormat, TextureView, VertexState};
use wgpu::BufferBindingType::{Storage, Uniform};
use wgpu::Face::Back;
use wgpu::FrontFace::Ccw;
use wgpu::PolygonMode::Fill;
use wgpu::PrimitiveTopology::TriangleList;
use wgpu::util::DeviceExt;
use crate::app::{App, AppVariant, ShaderType};
use crate::assets_helper::ResourceManager;
use crate::input_event::{EventType, InputEvent};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 2],
    predicted_pos: [f32; 2],
    velocity: [f32; 2],
    target_pos: [f32; 2],
    target_color: [f32; 3],
    density: f32,
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

    target_density: f32,
    pressure_multiplier: f32,
    pointer_location: [f32; 2],

    resolution: [f32; 2],
    pointer_active: f32,
    pointer_attract: f32,

    gravity_strength: f32,
    viscosity: f32,
    animate_strength: f32,
    _padding: f32
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BitonicSortParams {
    group_width: u32,
    group_height: u32,
    step_index: u32,
}

struct Renderer {
    queue: Queue,
    constants_buffer: wgpu::Buffer,
    constants_bg: wgpu::BindGroup,
    particles_buffer: wgpu::Buffer,
    particle_read_bg: wgpu::BindGroup,
    draw_particles_pso: RenderPipeline,

    bg_bg: wgpu::BindGroup,
    particle_read_write_bg: wgpu::BindGroup,
    voronoi_pso: RenderPipeline,
    particles_pre_update_pso: ComputePipeline,
    compute_particle_densities_pso: ComputePipeline,
    apply_particles_pressure_pso: ComputePipeline,
    update_particles_positions_pso: ComputePipeline,
    update_clr_and_target_pos_pso: ComputePipeline,
    apply_viscosity_pso: ComputePipeline,
    animate_pso: ComputePipeline,

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
    sort_params: BitonicSortParams,
    should_update_colors_and_pos: bool,
    animate: bool
}

impl Liquid2DExample {
    const SOLVER_FPS: f32 = 30f32;
    const SOLVER_DELTA_TIME: f32 = 1f32 / Self::SOLVER_FPS;
    const PARTICLES_CNT: usize = 3000;
    const WORKGROUP_SIZE: usize = 256;
    const WORKGROUP_CNT: u32 = ((Self::PARTICLES_CNT + Self::WORKGROUP_SIZE - 1) / Self::WORKGROUP_SIZE) as u32;
    //const SIM_BOUNDS: [f32; 2] = [12., 9.];
    const SIM_BOUNDS: [f32; 2] = [7.32, 9.];
    const PARTICLE_RADIUS: f32 = 0.065;
    const DEFAULT_GRAVITY: [f32; 2] = [0.0, -9.8];
    const DEFAULT_PARTICLE_MASS: f32 = 1.;
    const DEFAULT_PARTICLE_SEGMENTS: u32 = 24;
    const DEFAULT_DAMPING: f32 = 0.95;
    const DEFAULT_SMOOTHING_RADIUS: f32 = 0.35;
    const DEFAULT_TARGET_DENSITY: f32 = 1.5;
    const DEFAULT_PRESSURE_MULTIPLIER: f32 = 5.;
    const DEFAULT_VISCOSITY: f32 = 0.35;
}

impl<T: ResourceManager> App<T> for Liquid2DExample {
    fn get_extra_device_features(app_variant: AppVariant) -> Features {
        Features::PUSH_CONSTANTS | Features::SPIRV_SHADER_PASSTHROUGH
    }

    fn new(sc: &SurfaceConfiguration, adapter: &Adapter, device: &Device, queue: Queue, shader_type: ShaderType, resource_manager: &T) -> Self {
        println!("Adapter: {:?}", adapter.get_info());
        println!("Device's max push constant size: {}", device.limits().max_push_constant_size);
        println!("Adapter's max push constant size: {}", adapter.limits().max_push_constant_size);

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
                density: 1e-10,
                predicted_pos: [0.0; 2],
                target_pos: [0.0; 2],
                target_color: [1.0; 3],
            }
        }).collect::<Vec<_>>().try_into().unwrap();

        let (bg_tv, bg_sampler) = Self::create_bg(device, &queue);
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
        let bg_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Background Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }
            ],
        });
        let bg_bg = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Background Bind Group"),
            layout: &bg_bgl,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bg_tv),
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&bg_sampler),
                }
            ],
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
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
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
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
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
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
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
            viscosity: Self::DEFAULT_VISCOSITY,
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

        let (particles_pre_update_pso, compute_particle_densities_pso,
             apply_particles_pressure_pso, update_particles_positions_pso,
            animate_pso, apply_viscosity_pso) = Self::create_particles_update_psos(device, &constants_bgl, &particle_data_write_bgl);
        let update_clr_and_target_pos_pso = Self::create_particles_update_clr_and_target_pos(device, &constants_bgl, &particle_data_write_bgl, &bg_bgl);
        let draw_particles_pso = Self::create_draw_particles_pso(device, sc.format, shader_type, &constants_bgl, &particle_data_read_bgl);
        let overlay_pso = Self::create_overlay_pso(device, sc.format, &constants_bgl, &particle_data_write_bgl, &bg_bgl);
        let compute_spatial_lookup_cp = Self::create_spatial_lookup_pso(device, &constants_bgl, &spatial_lookup_bgl);
        let write_start_indices_cp = Self::create_start_indices_pso(device, &constants_bgl, &spatial_lookup_bgl);
        let sort_lookup_cp = Self::create_sort_lookup_pso(device, &constants_bgl, &spatial_lookup_bgl);

        let renderer = Renderer{ queue, constants_buffer, constants_bg, particles_buffer,
                                 particle_read_bg, draw_particles_pso, bg_bg, particle_read_write_bg,
            voronoi_pso: overlay_pso,
                                 particles_pre_update_pso, compute_particle_densities_pso, apply_particles_pressure_pso,
                                 update_particles_positions_pso, update_clr_and_target_pos_pso, apply_viscosity_pso, animate_pso,
                                 spatial_lookup_bg, compute_spatial_lookup_cp, write_start_indices_cp, sort_lookup_cp,
                                 imgui_context, imgui_renderer };
        Liquid2DExample{ renderer, constants, sort_params: BitonicSortParams::default(), should_update_colors_and_pos: false, animate: false }
    }

    fn tick(&mut self, device: &Device, delta: f32) {
        self.constants.delta_time += delta;
        self.renderer.imgui_context.io_mut().update_delta_time(Duration::from_secs_f32(delta));

        if self.constants.delta_time > Self::SOLVER_DELTA_TIME {
            self.solver_step(device);
        }
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
        match event.event_type {
            EventType::Move => {
                self.renderer.imgui_context.io_mut().mouse_pos = [event.coords[0], event.coords[1]];
                self.constants.pointer_location = event.coords;
            },
            EventType::Start(btn) => {
                if btn == 2 {
                    self.should_update_colors_and_pos = true;
                } else {
                    self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = true;
                    self.constants.pointer_active = 1.0;
                    self.constants.pointer_attract = btn as f32;
                }
            },
            EventType::End => {
                self.renderer.imgui_context.io_mut().mouse_down[0 as usize] = false;
                self.constants.pointer_active = 0.0;
            },
            EventType::Wheel(delta) => {
                self.constants.animate_strength += (delta * 0.2 - 0.1);
                self.constants.animate_strength = self.constants.animate_strength.max(0f32);
            }
            EventType::None => (),
        };
        false
    }

    fn render(&mut self, frame: &Surface, device: &Device) -> Result<(), SurfaceError> {
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
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None
            });

            render_pass.set_pipeline(&self.renderer.voronoi_pso);
            render_pass.set_bind_group(0, &self.renderer.constants_bg, &[]);
            render_pass.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            render_pass.set_bind_group(2, &self.renderer.bg_bg, &[]);
            render_pass.draw(0..3, 0..1);

            /*render_pass.set_pipeline(&self.renderer.draw_particles_pso);
            render_pass.set_bind_group(0, &self.renderer.constants_bg, &[]);
            render_pass.set_bind_group(1, &self.renderer.particle_read_bg, &[]);
            render_pass.draw(0..Self::PARTICLES_CNT as u32 * 3 * self.constants.particle_segments, 0..1);*/

            let ui = self.renderer.imgui_context.frame();
            ui.window("Settings")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.slider("Smoothing rad", 0.01, 3., &mut self.constants.smoothing_radius);
                    ui.slider("Particle mass", 0.01, 10., &mut self.constants.particle_mass);
                    ui.slider("Target density", 0., 10., &mut self.constants.target_density);
                    ui.slider("Pressure multiplier", 0., 30., &mut self.constants.pressure_multiplier);
                    ui.slider("Gravity", 0., 1., &mut self.constants.gravity_strength);
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
    fn create_draw_particles_pso(device: &Device, tex_format: TextureFormat, shader_type: ShaderType, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> RenderPipeline {
        let mut spirv_modules : Vec<ShaderModule> = vec![];
        unsafe {
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/particle.vs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/particle.fs.spv")));
        };

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Visualize Particle Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor{
            label: Some("Draw Particles Pipeline"),
            layout: Some(&rpl),
            vertex: VertexState {
                module: &spirv_modules[0],
                entry_point: "main",
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
                module: &spirv_modules[1],
                entry_point: "main",
                targets: &[Some(ColorTargetState{
                    format: tex_format,
                    blend: Default::default(),
                    write_mask: Default::default(),
                })],
            }),
            multiview: None,
        })
    }

    fn create_particles_update_psos(device: &Device, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout) -> (ComputePipeline, ComputePipeline, ComputePipeline, ComputePipeline, ComputePipeline, ComputePipeline) {
        let mut spirv_modules = vec![];
        unsafe {
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/update_predicted_pos.cs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/update_densities.cs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/apply_pressure.cs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/update_positions.cs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/animate.cs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/apply_viscosity.cs.spv")));
        }

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Process Particles Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl],
            push_constant_ranges: &[],
        });

        (
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Particles PreUpdate Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[0],
                entry_point: "main",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Calculate Particle Densities Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[1],
                entry_point: "main",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Apply Particles Pressure Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[2],
                entry_point: "main",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Update Particles Positions Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[3],
                entry_point: "main",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Animate Particles Positions Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[4],
                entry_point: "main",
            }),
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
                label: Some("Apply viscosity Pipeline"),
                layout: Some(&rpl),
                module: &spirv_modules[5],
                entry_point: "main",
            }),
        )
    }

    fn create_particles_update_clr_and_target_pos(device: &Device, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout, bg_bgl: &BindGroupLayout) -> (ComputePipeline) {
        let mut spirv_modules = vec![];
        unsafe {
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/update_color_and_target_pos.cs.spv")));
        }

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Update Particles Clr Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl, bg_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Animate particle pipeline"),
            layout: Some(&rpl),
            module: &spirv_modules[0],
            entry_point: "main",
        })
    }

    fn create_spatial_lookup_pso(device: &Device, constants_bgl: &BindGroupLayout, /*sort_params_bgl: &BindGroupLayout,*/spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let mut spirv_module = vec![];
        unsafe {
            spirv_module.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/write_spatial_lookup.cs.spv")));
        }

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Create spatial lookup pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Create spatial lookup Pipeline"),
            layout: Some(&rpl),
            module: &spirv_module[0],
            entry_point: "main",
        })
    }

    fn create_start_indices_pso(device: &Device, constants_bgl: &BindGroupLayout, /*sort_params_bgl: &BindGroupLayout, */spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let mut spirv_module = vec![];
        unsafe {
            spirv_module.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/write_start_indices.cs.spv")));
        }

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Write start indices pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("write start indices Pipeline"),
            layout: Some(&rpl),
            module: &spirv_module[0],
            entry_point: "main",
        })
    }

    fn create_sort_lookup_pso(device: &Device, constants_bgl: &BindGroupLayout, /*sort_params_bgl: &BindGroupLayout, */spatial_lookup_bgl: &BindGroupLayout) -> ComputePipeline {
        let mut spirv_module = vec![];
        unsafe {
            spirv_module.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/sort_pairs.cs.spv")));
        }

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Sort lookup pipeline Layout"),
            bind_group_layouts: &[constants_bgl, spatial_lookup_bgl],
            push_constant_ranges: &[PushConstantRange{
                stages: ShaderStages::COMPUTE,
                range: 0..size_of::<BitonicSortParams>() as u32,
            }],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Sort lookup pipeline"),
            layout: Some(&rpl),
            module: &spirv_module[0],
            entry_point: "main",
        })
    }

    fn create_overlay_pso(device: &Device, tex_format: TextureFormat, constants_bgl: &BindGroupLayout, particle_data_bgl: &BindGroupLayout, bg_bgl: &BindGroupLayout) -> RenderPipeline {
        let mut spirv_modules : Vec<ShaderModule> = vec![];
        unsafe {
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/voronoi.vs.spv")));
            spirv_modules.push(device.create_shader_module_spirv(&include_spirv_raw!("shaders/spirv/voronoi.fs.spv")));
        };

        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Process Particles Pipeline Layout"),
            bind_group_layouts: &[constants_bgl, particle_data_bgl/*, bg_bgl*/],
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
                module: &spirv_modules[0],
                entry_point: "main",
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
                module: &spirv_modules[1],
                entry_point: "main",
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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label: Some("Solver Step Encoder")
        });

        if self.should_update_colors_and_pos {
            //encoder.push_debug_group("Update partilce colors");
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Update Particle Colors Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.update_clr_and_target_pos_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.set_bind_group(2, &self.renderer.bg_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
            //encoder.pop_debug_group();
            self.should_update_colors_and_pos = false;
            self.animate = true;
        }

        encoder.push_debug_group("Predict particle positions");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Update Particle Velocities Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.particles_pre_update_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();


        encoder.push_debug_group("Apply viscosity");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Apply viscosity Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.apply_viscosity_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();


        if self.animate {
            //encoder.push_debug_group("Animate particles");
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Animate Particle Positions Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.animate_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
            //encoder.pop_debug_group();
        }

        encoder.push_debug_group("Write spatial lookup");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Write spatial lookup compute pass"),
            });
            cp.set_pipeline(&self.renderer.compute_spatial_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("Bitonic sort");
        {
            let num_pairs = Self::next_power_of_two(Self::PARTICLES_CNT as u32) / 2;
            let num_stages = ((num_pairs * 2) as f32).log(2.0) as u32;

            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Bitonic steps compute pass"),
            });
            cp.set_pipeline(&self.renderer.sort_lookup_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);

            for stage_index in 0..num_stages {
                for step_index in 0..(stage_index + 1) {
                    let group_width = 1 << (stage_index - step_index);
                    let group_height = 2 * group_width - 1;
                    self.sort_params.group_width = group_width;
                    self.sort_params.group_height = group_height;
                    self.sort_params.step_index = step_index;
                    cp.set_push_constants(0, bytemuck::cast_slice(&[self.sort_params]));
                    cp.dispatch_workgroups(num_pairs / Self::WORKGROUP_SIZE as u32, 1, 1);
                }
            }
        }
        encoder.pop_debug_group();
        encoder.push_debug_group("Write start indices");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Write start indices compute pass"),
            });
            cp.set_pipeline(&self.renderer.write_start_indices_cp);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.spatial_lookup_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();
        encoder.push_debug_group("Calculate densities");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Calculate Particle Densities Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.compute_particle_densities_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();
        encoder.push_debug_group("Apply pressure");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Apply Particle pressure Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.apply_particles_pressure_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();

        encoder.push_debug_group("Update particle positions");
        {
            let mut cp = encoder.begin_compute_pass(&ComputePassDescriptor{
                label: Some("Update Particle positions Compute Pass"),
            });
            cp.set_pipeline(&self.renderer.update_particles_positions_pso);
            cp.set_bind_group(0, &self.renderer.constants_bg, &[]);
            cp.set_bind_group(1, &self.renderer.particle_read_write_bg, &[]);
            cp.dispatch_workgroups(Self::WORKGROUP_CNT, 1, 1);
        }
        encoder.pop_debug_group();
        self.renderer.queue.submit(iter::once(encoder.finish()));
        self.constants.delta_time = 0f32;
    }

    fn create_bg(device: &Device, queue: &Queue) -> (TextureView, Sampler) {
        let bg_image = image::load_from_memory(include_bytes!("../../../assets/textures/de-chirico-canto-d-amore.jpg")).unwrap();
        let bg_rgba = bg_image.to_rgba8();

        let bg_size = wgpu::Extent3d {
            width: bg_image.width(),
            height: bg_image.height(),
            depth_or_array_layers: 1,
        };
        let bg_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: bg_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("TheLovers2 texture"),
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &bg_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bg_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * bg_image.width()),
                rows_per_image: Some(bg_image.height()),
            },
            bg_size,
        );

        let bg_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bg_texture_view = bg_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (bg_texture_view, bg_sampler)
    }
}