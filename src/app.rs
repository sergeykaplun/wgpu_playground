use wgpu::Queue;

use crate::app_variants::ShaderType;

pub trait App {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType
    ) -> Self;
    fn resize(&mut self, sc: &wgpu::SurfaceConfiguration, device: &wgpu::Device);
    fn tick(&mut self, delta: f32);
    fn render(&mut self, frame: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError>;
}