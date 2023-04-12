use std::iter;
use wgpu::Queue;
use crate::{app::App, app::ShaderType, assets_helper::ResourceManager, camera::{ArcballCamera, Camera}, input_event::InputEvent, skybox::{Skybox, Drawable}};

pub struct Renderer {
    queue: Queue,
}

pub struct SkyboxExample {
    renderer : Renderer,
    skybox: Skybox,
    camera: ArcballCamera,
}

impl<T: ResourceManager> App<T> for SkyboxExample{
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType,
        resource_manager: &T
    ) -> Self {
        let camera = ArcballCamera::new(&device, sc.width as f32, sc.height as f32, 45., 0.01, 100., 7., 35.);
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
        let skybox = Skybox::new(device, &queue, resource_manager, sc.format, shader_type, &camera_bind_group_layout);
        Self{
            renderer: Renderer { queue },
            skybox: skybox,
            camera
        }
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
            
            render_pass.draw_skybox(&self.skybox, &self.camera.camera_bind_group);
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
        self.camera.tick(delta, &self.renderer.queue)
    }
}