use std::{f32::consts::PI, mem};

use wgpu::{Buffer, BindGroup, Device, util::DeviceExt, Queue, BindGroupLayoutEntry, BindGroupEntry, BufferUsages, BufferDescriptor};
use crate::input_event::{InputEvent, EventType};

pub trait Camera {
    fn input(&mut self, event: &InputEvent);
    fn tick(&mut self, time_delta: f32, queue: &Queue);
}

pub struct ArcballCamera {
    width: f32,
    height: f32,
    fov: f32,
    znear: f32,
    zfar: f32,
    speed: f32,

    camera_buffer: Buffer,
    pub camera_bind_group: BindGroup,

    dist: f32,
    pub azimuth: f32,
    pub polar: f32,

    prev_input_event: InputEvent,
}

impl ArcballCamera {
    pub fn new(device: &Device, width: f32, height: f32,
                fov: f32, znear: f32, zfar: f32, speed: f32, dist: f32) -> Self {
        let arr = [0.0f32; 64];
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[arr]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
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

            camera_buffer,
            camera_bind_group,

            dist,
            azimuth: 0.,
            polar: 0.,

            prev_input_event: InputEvent::default(),
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
    fn input(&mut self, new_event: &InputEvent) {
        let event = InputEvent::diff(&self.prev_input_event, &new_event);
        match &event.event_type {
            EventType::Move => {
                self.azimuth -= event.coords[0] / self.width * self.speed;
                self.polar   -= event.coords[1] / self.height * self.speed;
                self.polar = self.polar.clamp(-PI * 0.35, PI * 0.35);
            },
            _ => (),
        }
        self.prev_input_event = new_event.clone();
    }

    fn tick(&mut self, time_delta: f32, queue: &Queue) {
        let mut eye = glm::vec3::<f32>(0., 0., self.dist);
        eye = glm::rotate_x_vec3(&eye, self.polar);
        eye = glm::rotate_y_vec3(&eye, self.azimuth);

        let proj = glm::perspective_fov(self.fov, self.width, self.height, self.znear, self.zfar);
        let view = glm::look_at(&eye, &glm::vec3::<f32>(0., -0.125, 0.), &glm::vec3::<f32>(0., 1., 0.));
        // let mat = glm::perspective_fov(self.fov, self.width, self.height, self.znear, self.zfar) * 
        //                                               glm::look_at(&eye, &glm::vec3::<f32>(0., -0.125, 0.), &glm::vec3::<f32>(0., 1., 0.));
        
        //let mat = glm::ortho(-2.0, 2.0, -1.0, 1.0, -1.0, 1.0)
        //                                            * glm::look_at(&eye, &glm::Vec3::zeros(), &glm::vec3::<f32>(0., 1., 0.));
        
        // struct CameraParams {
        //     projection :                      mat4x4<f32>,
        //     model :                           mat4x4<f32>,
        //     view :                            mat4x4<f32>,
        //     position :                        vec3<f32>,
        //   };
        
        let mut uniform = Vec::<f32>::new();
        uniform.extend(proj.iter());
        uniform.extend(glm::Mat4::identity().iter());
        uniform.extend(view.iter());
        // uniform.extend(mat.iter());
        uniform.extend(eye.iter());
        uniform.push(1.0);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&uniform));
    }
}