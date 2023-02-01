use wgpu::Queue;

pub trait App {
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
    ) -> Self;
    fn resize(&mut self, sc: &wgpu::SurfaceConfiguration, device: &wgpu::Device);
    fn tick(&mut self, delta: f32);
    fn render(&mut self, frame: &wgpu::Surface, device: &wgpu::Device) -> Result<(), wgpu::SurfaceError>;
}