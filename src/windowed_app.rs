use std::{collections::VecDeque, path::Path};
use std::time::{Duration, Instant};

use wgpu::{InstanceDescriptor, Backends, RequestAdapterOptions, Limits, DeviceDescriptor, TextureUsages, SurfaceConfiguration};
use winit::{event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}, window::Icon};
use winit::dpi::Size;

use crate::{app::{App, AppVariant}, assets_helper::DesktopResourceManager, input_event::InputEvent};

pub async fn run<T: App<DesktopResourceManager> + 'static>(title: &str, app_variant: AppVariant) {
    env_logger::init();
    
    let event_loop = EventLoop::new();
    let icon = match image::open(Path::new("./assets/logo.png")) {
        Ok(file) => {
            Some(file.to_rgba8())
        },
        Err(error) => {
            println!("Failed to open icon asset - {}", error);
            None
        },
    };
    let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080)).with_title(title).build(&event_loop).unwrap();
    if let Some(icon) = icon {
        let (icon_width, icon_height) = icon.dimensions();
        window.set_window_icon(Some(Icon::from_rgba(icon.clone().into_raw(), icon_width, icon_height).unwrap()))
    }

    let size = window.inner_size();
    let instance = wgpu::Instance::new(InstanceDescriptor{
        //backends: Backends::all(),
        backends: Backends::DX12,
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
    });
    let surface = unsafe{ instance.create_surface(&window).ok().unwrap() };
    let adapter = instance.request_adapter(&RequestAdapterOptions{
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }).await.unwrap();

    println!(
        "Device: {}",
        adapter.get_info().name
    );
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                features: T::get_extra_device_features(app_variant),
                limits: Limits::default()
            },
            None
        ).await.unwrap();
    let caps = surface.get_capabilities(&adapter);
    let mut surface_config = SurfaceConfiguration { 
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: caps.formats[0],
        width: size.width, 
        height: size.height, 
        present_mode: caps.present_modes[0], 
        alpha_mode: caps.alpha_modes[0], 
        view_formats: vec![caps.formats[0]]
    };
    surface.configure(&device, &surface_config);

    let mut app_instance = T::new(&surface_config, &adapter, &device, queue, app_variant.shader_type, &DesktopResourceManager{});

    let mut moment = std::time::Instant::now();
    let mut fps_data = VecDeque::new();
    let mut latest_fps_print = std::time::Instant::now();
    let mut input_event = InputEvent::default();

    const TARGET_FPS: u32 = 240;
    let frame_duration = Duration::from_secs(1) / TARGET_FPS;
    let mut last_frame_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let duration = moment.elapsed();
        let delta = duration.as_secs_f32() + (duration.subsec_nanos() as f32 * 1.0e-9);
        if fps_data.len() > 1000 {
            fps_data.pop_front();
        }
        fps_data.push_back(delta);
        moment = std::time::Instant::now();
        
        // TODO this isn't fps. redo
        if latest_fps_print.elapsed().as_secs_f32() > 1.0 {
            println!(
                "Avg fps: {}",
                1.0 / (fps_data.iter().sum::<f32>() / fps_data.len() as f32)
            );
            latest_fps_print = std::time::Instant::now();
        }

        app_instance.tick(delta);
        
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let new_event = InputEvent::from_winit_event(event);
                //if !app_instance.process_input(&InputEvent::diff(&input_event, &new_event)) {
                if !app_instance.process_input(&new_event) {
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            surface_config.width = physical_size.width;
                            surface_config.height = physical_size.height;
                            surface.configure(&device, &surface_config);

                            app_instance.resize(&surface_config, &device);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            app_instance.resize(&surface_config, &device);
                        },
                        _ => {}
                    }
                }
                input_event = new_event;
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = Instant::now();
                let elapsed_time = now - last_frame_time;

                //if elapsed_time >= frame_duration
                {
                    last_frame_time = now;
                    match app_instance.render(&surface, &device) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => app_instance.resize(&surface_config, &device),
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                    };
                }
            }
            _ => {}
        }
    });
}