use wgpu::Queue;
use structopt::StructOpt;
use std::str::FromStr;

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

#[derive(StructOpt, Debug, Clone, Copy)]
#[structopt(name = "settings")]
pub struct AppVariant {
    #[structopt(short = "s", long = "shader_type", default_value = "WGSL")]
    pub(crate) shader_type: ShaderType,
}

#[derive(Debug, Copy, Clone)]
pub enum ShaderType {
    WGSL,
    SPIRV
}

type ParseError = &'static str;
impl FromStr for ShaderType {
    type Err = ParseError;
    fn from_str(input: &str) -> Result<ShaderType, Self::Err> {
        match input {
            "wgsl" | "WGSL" => Ok(ShaderType::WGSL),
            "spirv" | "SPIRV" => Ok(ShaderType::SPIRV),
            _ => Err("Could not parse a ShaderType"),
        }
    }
}

impl std::fmt::Display for ShaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderType::WGSL => write!(f, "WGSL"),
            ShaderType::SPIRV => write!(f, "SPIRV"),
        }
    }
}