use std::str::FromStr;
use structopt::StructOpt;
use wgpu::{Features, Queue};

use crate::input_event::InputEvent;
use crate::assets_helper::ResourceManager;

pub trait App<T: ResourceManager> {
    fn get_extra_device_features(app_variant: AppVariant) -> Features {
        match app_variant.shader_type {
            ShaderType::WGSL => Features::empty(),
            ShaderType::SPIRV => Features::SPIRV_SHADER_PASSTHROUGH,
        }
    }
    fn new(
        sc: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: Queue,
        shader_type: ShaderType,
        resource_manager: &T
    ) -> Self;

    fn process_input(&mut self, _event: &InputEvent) -> bool {
        false
    }

    fn resize(&mut self, _sc: &wgpu::SurfaceConfiguration, _device: &wgpu::Device) {}
    fn tick(&mut self, _delta: f32) {}
    fn render(
        &mut self,
        frame: &wgpu::Surface,
        device: &wgpu::Device,
    ) -> Result<(), wgpu::SurfaceError>;
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
    SPIRV,
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
