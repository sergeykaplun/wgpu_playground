use std::f32::consts::PI;

use wgpu::{Buffer, BindGroup, Device, util::DeviceExt, Queue, BindGroupLayoutEntry, BindGroupEntry};
use winit::{event::{WindowEvent, ElementState, MouseButton, MouseScrollDelta}, dpi::PhysicalPosition};

pub trait Camera {
    fn input(&mut self, event: &WindowEvent) -> bool;
    fn tick(&mut self, time_delta: f32, queue: &Queue);
}

pub struct ArcballCamera {
    width: f32,
    height: f32,
    fov: f32,
    znear: f32,
    zfar: f32,
    speed: f32,

    pub view_proj_mat: [[f32; 4]; 4],
    time_in_flight: f32,

    camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,

    mouse_last_coord: PhysicalPosition<f64>,
    mouse_pressed: bool,

    dist: f32,
    azimuth: f32,
    polar: f32,
}

impl ArcballCamera {
    pub fn new(device: &Device, width: f32, height: f32,
                fov: f32, znear: f32, zfar: f32, speed: f32, dist: f32) -> Self {
        let arr: [[f32; 4]; 4] = glm::Mat4::identity().into();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            label: Some("camera_bind_group_layout"),
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        
        Self{
            width,
            height,
            fov,
            znear,
            zfar,
            speed,

            view_proj_mat: arr,
            time_in_flight: 0.0,

            camera_buffer,
            camera_bind_group,

            mouse_last_coord: PhysicalPosition { x: 0., y: 0. },
            mouse_pressed: false,

            dist,
            azimuth: 0.,
            polar: 0.,
        }
    }

    pub fn get_bind_group(&self, index: u32) -> (BindGroupLayoutEntry, BindGroupEntry) {
        (
            wgpu::BindGroupLayoutEntry {
                binding: index,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupEntry {
                binding: index,
                resource: self.camera_buffer.as_entire_binding(),
            }
        )
    }
}

impl Camera for ArcballCamera {
    fn input(&mut self, event: &WindowEvent) -> bool{
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    self.azimuth -= ((position.x - self.mouse_last_coord.x) as f32) / self.width * self.speed;
                    self.polar -= ((position.y - self.mouse_last_coord.y) as f32) / self.height * self.speed;
                    
                    self.polar = self.polar.clamp(-PI * 0.35, 0.0);
                }

                self.mouse_last_coord.clone_from(position);
                false
            },
            WindowEvent::MouseInput { button, state, .. } => {
                if let MouseButton::Left = button {
                    self.mouse_pressed = state.clone() == ElementState::Pressed;
                }
                false
            },
            WindowEvent::MouseWheel { delta, ..} => {
                if let MouseScrollDelta::LineDelta(_x, y) = delta {
                    self.dist -= y * 0.1 * self.speed;
                }
                false
            },
            _ => false,
        }
    }

    fn tick(&mut self, time_delta: f32, queue: &Queue) {
        let mut eye = glm::vec3::<f32>(0., 0., self.dist);
        eye = glm::rotate_x_vec3(&eye, self.polar);
        eye = glm::rotate_y_vec3(&eye, self.azimuth);

        let mat = glm::perspective_fov(self.fov, self.width, self.height, self.znear, self.zfar) * 
                                                      glm::look_at(&eye, &glm::Vec3::zeros(), &glm::vec3::<f32>(0., 1., 0.));
        
        //let mat = glm::ortho(-2.0, 2.0, -1.0, 1.0, -1.0, 1.0)
        //                                            * glm::look_at(&eye, &glm::Vec3::zeros(), &glm::vec3::<f32>(0., 1., 0.));
        
        self.view_proj_mat = mat.into();
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.view_proj_mat]));

        self.time_in_flight += time_delta;
    }
}