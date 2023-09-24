use std::iter;
use wgpu::{Adapter, BindGroup, BindGroupLayout, BindGroupLayoutEntry, BindingType, BlendState, BufferUsages, ColorTargetState, ComputePipeline, Device, Extent3d, Features, FragmentState, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderSource, Surface, SurfaceConfiguration, SurfaceError, Texture, TextureFormat, TextureSampleType, TextureView, TextureViewDimension, VertexState};
use wgpu::AddressMode::ClampToEdge;
use wgpu::Face::Back;
use wgpu::FilterMode::Linear;
use wgpu::FrontFace::Ccw;
use wgpu::PolygonMode::Fill;
use wgpu::PrimitiveTopology::TriangleList;
use wgpu::util::DeviceExt;
use wgpu_profiler::{GpuProfiler, GpuTimerScopeResult, wgpu_profiler};
use crate::app::{App, AppVariant, ShaderType};
use crate::assets_helper::ResourceManager;
use crate::camera::{ArcballCamera, Camera};
use crate::geometry_primitives::{CUBE_DATA, CUBE_INDICES, CUBE_VBL};
use crate::input_event::InputEvent;
use crate::skybox::{DrawableSkybox, Skybox};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    time: f32,
    volume_resolution: u32,
}

struct Renderer {
    pipeline: RenderPipeline,
    volume_read_bind_groups: [BindGroup; 2],
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    queue: Queue,

    emit_bubbles_pipeline: ComputePipeline,
    update_volume_pipeline: ComputePipeline,
    volume_write_bind_groups: [BindGroup; 2],

    constants_buffer: wgpu::Buffer,
    constants_bind_group: BindGroup,
}

pub struct VolumetricExample {
    renderer: Renderer,
    camera: ArcballCamera,
    skybox: Skybox,
    constants: Constants,
    frame_index: u32,
    profiler: GpuProfiler,
}

impl<T: ResourceManager> App<T> for VolumetricExample {
    fn get_extra_device_features(_app_variant: AppVariant) -> Features {
        Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES |
        Features::TIMESTAMP_QUERY |
        Features::TIMESTAMP_QUERY_INSIDE_PASSES
    }

    fn new(sc: &SurfaceConfiguration, adapter: &Adapter, device: &Device, queue: Queue, shader_type: ShaderType, res_manager: &T) -> Self {
        let profiler = GpuProfiler::new(&adapter, &device, &queue, 4);
        let camera = ArcballCamera::new(device, sc.width as f32, sc.height as f32, 45., 0.1, 1000., 4., 2.5);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volumetric Vertex Buffer"),
            contents: bytemuck::cast_slice(&CUBE_DATA),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volumetric Index Buffer"),
            contents: bytemuck::cast_slice(&CUBE_INDICES),
            usage: BufferUsages::INDEX,
        });
        let num_indices = CUBE_INDICES.len() as u32;
        let skybox = Skybox::new(device, &queue, res_manager,sc.format, shader_type, &camera.bgl, false);
        let (_volumetric_data, volumetric_data_views, volumetric_data_sampler) = Self::create_storage_texture(device, Self::VOLUME_EXTENT, TextureFormat::Rgba16Float);
        let write_volume_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Write volume bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba16Float,
                    view_dimension: TextureViewDimension::D3,
                },
                count: None,
            }],
        });
        let volume_write_bind_groups: [BindGroup; 2] = volumetric_data_views.iter().enumerate().map(|v|
            device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                    label: Some(format!("Volume write bind group {}", v.0).as_str()),
                    layout: &write_volume_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(v.1),
                        }
                    ],
                }
            )
        ).collect::<Vec<_>>().try_into().unwrap();

        let volume_read_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Volume read bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,

                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: TextureViewDimension::D3,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Sampler {
                        0: SamplerBindingType::Filtering,
                    },
                    count: None,
                }
            ],
        });
        let volume_read_bind_groups: [BindGroup; 2] = volumetric_data_views.iter().enumerate().map(|v|
            device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                        label: Some(format!("Volume read bind group {}", v.0).as_str()),
                        layout: &volume_read_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(v.1),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&volumetric_data_sampler),
                            }
                        ],
                    }
            )
        ).collect::<Vec<_>>().try_into().unwrap();

        let constants = Constants { time: 0.0, volume_resolution: Self::VOLUME_EXTENT.width};
        let constants_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Constants Buffer"),
            size: std::mem::size_of::<Constants>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let constants_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Constants Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let constants_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Constants Bind Group"),
            layout: &constants_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: constants_buffer.as_entire_binding(),
            }],
        });

        let emit_bubbles_pipeline =  VolumetricExample::create_emit_bubbles_compute_pipeline(device, &write_volume_bgl, &constants_bind_group_layout);
        let update_volume_pipeline =  VolumetricExample::create_update_volume_compute_pipeline(device, &write_volume_bgl, &volume_read_bgl);
        let pipeline = VolumetricExample::create_volumetric_pipeline(device, sc.format, &camera.bgl, &volume_read_bgl);
        let renderer = Renderer {pipeline, volume_read_bind_groups, vertex_buffer, index_buffer, num_indices, queue, emit_bubbles_pipeline, update_volume_pipeline, volume_write_bind_groups, constants_buffer, constants_bind_group};

        VolumetricExample { renderer, camera, skybox, constants, frame_index: 0, profiler }
    }

    fn process_input(&mut self, event: &InputEvent) -> bool {
        self.camera.input(event);
        false
    }

    fn tick(&mut self, delta: f32) {
        self.constants.time += delta;
    }

    fn render(&mut self, surface: &Surface, device: &Device) -> Result<(), SurfaceError> {
        self.camera.tick(0.01, &self.renderer.queue);
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let emit_index = (self.frame_index % 2) as usize;
        let update_index = ((self.frame_index + 1) % 2) as usize;
        self.frame_index += 1;
        encoder.push_debug_group("Update volume pass");
        wgpu_profiler!("Update volume pass", &mut self.profiler, &mut encoder, &device, {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Update volume pass") });
            cpass.set_pipeline(&self.renderer.update_volume_pipeline);
            cpass.set_bind_group(0, &self.renderer.volume_write_bind_groups[emit_index], &[]);
            cpass.set_bind_group(1, &self.renderer.volume_read_bind_groups[update_index], &[]);
            cpass.dispatch_workgroups(Self::VOLUME_EXTENT.width/4, Self::VOLUME_EXTENT.height/4, Self::VOLUME_EXTENT.depth_or_array_layers/4);
        });
        encoder.pop_debug_group();

        encoder.push_debug_group("Emit bubbles pass");
        wgpu_profiler!("Emitter pass", &mut self.profiler, &mut encoder, &device, {
            self.renderer.queue.write_buffer(&self.renderer.constants_buffer, 0, bytemuck::cast_slice(&[self.constants]));
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Emit bubbles pass") });
            cpass.set_pipeline(&self.renderer.emit_bubbles_pipeline);
            cpass.set_bind_group(0, &self.renderer.volume_write_bind_groups[emit_index], &[]);
            cpass.set_bind_group(1, &self.renderer.constants_bind_group, &[]);
            cpass.dispatch_workgroups(Self::VOLUME_EXTENT.width/4, Self::VOLUME_EXTENT.height/4, Self::VOLUME_EXTENT.depth_or_array_layers/4);
        });
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
                depth_stencil_attachment: None,
            });

            //wgpu_profiler!("Draw skybox", &mut self.profiler, &mut encoder, &device, {
                render_pass.draw_skybox(&self.skybox, &self.camera.camera_bind_group);
            //});

            //wgpu_profiler!("Raymarch pass", &mut self.profiler, &mut encoder, &device, {
                render_pass.set_pipeline(&self.renderer.pipeline);
                render_pass.set_bind_group(0, &self.camera.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.renderer.volume_read_bind_groups[emit_index], &[]);
                render_pass.set_vertex_buffer(0, self.renderer.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.renderer.num_indices, 0, 0..1);
            //});
        }
        self.profiler.resolve_queries(&mut encoder);

        //wgpu_profiler!("Queue submit", &mut self.profiler, &mut encoder, &device, {
            self.renderer.queue.submit(iter::once(encoder.finish()));
        //});
        //wgpu_profiler!("Queue present", &mut self.profiler, &mut encoder, &device, {
            output.present();
        //});

        //profiling::finish_frame!();

        // Signal to the profiler that the frame is finished.
        self.profiler.end_frame().unwrap();
        if let Some(results) = self.profiler.process_finished_frame() {
            Self::console_output(&results);
        }

        Ok(())
    }
}

impl VolumetricExample {
    const VOLUME_EXTENT: Extent3d = wgpu::Extent3d {
        width: 512,
        height: 512,
        depth_or_array_layers: 512,
    };
    fn create_volumetric_pipeline(device: &Device, tex_format: TextureFormat, camera_bgl: &BindGroupLayout, volume_bgl: &BindGroupLayout) -> RenderPipeline{
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Volumetric Shader Module"),
            source: ShaderSource::Wgsl(include_str!("./shaders/wgsl/volumetric.wgsl").into()),
        });
        let rpl = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Volumetric Pipeline Layout"),
            bind_group_layouts: &[camera_bgl, volume_bgl],
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
            label: Some("Volumetric Pipeline"),
            layout: Some(&rpl),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: CUBE_VBL,
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

    fn create_emit_bubbles_compute_pipeline(device: &Device, volume_write_bgl: &BindGroupLayout, constants_bgl: &BindGroupLayout) -> ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Emit bubbles compute pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Emit bubbles compute pipeline layout"),
                bind_group_layouts: &[volume_write_bgl, constants_bgl],
                push_constant_ranges: &[],
            })),
            module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Emit bubbles compute shader"),
                source: ShaderSource::Wgsl(include_str!("shaders/wgsl/emit_bubbles.wgsl").into()),
            }),
            entry_point: "emit",
        })
    }

    fn create_update_volume_compute_pipeline(device: &Device, update_volume_bgl: &BindGroupLayout, volume_read_bgl: &BindGroupLayout) -> ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Update volume compute pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Update volume compute pipeline layout"),
                bind_group_layouts: &[update_volume_bgl, volume_read_bgl],
                push_constant_ranges: &[],
            })),
            module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Update volume compute shader"),
                source: ShaderSource::Wgsl(include_str!("shaders/wgsl/update_volume.wgsl").into()),
            }),
            entry_point: "update_volume",
        })
    }

    fn create_storage_texture(device: &Device, size: Extent3d, format: TextureFormat) -> ([Texture; 2], [wgpu::TextureView; 2], wgpu::Sampler) {
        let textures: [Texture; 2] = (0..2).map(|i|
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(format!("Volumetric Storage Texture {}", i).as_str()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        ).collect::<Vec<_>>().try_into().unwrap();
        let views: [TextureView; 2] = textures.iter().map(|t|
            t.create_view(&wgpu::TextureViewDescriptor::default())
        ).collect::<Vec<_>>().try_into().unwrap();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volumetric data sampler"),
            address_mode_u: ClampToEdge,
            address_mode_v: ClampToEdge,
            address_mode_w: ClampToEdge,
            mag_filter: Linear,
            min_filter: Linear,
            mipmap_filter: Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1u16.into(),
            border_color: None,
        });
        (textures, views, sampler)
    }

    fn scopes_to_console_recursive(results: &[GpuTimerScopeResult], indentation: u32) {
        for scope in results {
            if indentation > 0 {
                print!("{:<width$}", "|", width = 4);
            }
            println!("{:.3}μs - {}", (scope.time.end - scope.time.start) * 1000.0 * 1000.0, scope.label);
            if !scope.nested_scopes.is_empty() {
                Self::scopes_to_console_recursive(&scope.nested_scopes, indentation + 1);
            }
        }
    }

    fn console_output(results: &Vec<GpuTimerScopeResult>) {
        print!("\x1B[2J\x1B[1;1H");
        println!("Frame profiler results:");
        Self::scopes_to_console_recursive(results, 0);
        println!();
    }
}