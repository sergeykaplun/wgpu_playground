use crate::{app::{App, ShaderType}, camera::{ArcballCamera, Camera}};

struct ShadowMappingExample {}

impl App for ShadowMappingExample {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: wgpu::Queue,
        shader_type: ShaderType
    ) -> Self {
        todo!()
    }

    fn render(&mut self, frame: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError> {
        todo!()
    }
}