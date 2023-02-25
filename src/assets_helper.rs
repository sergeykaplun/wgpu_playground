use std::io::{BufReader, Cursor};

use tobj::Model;

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) {
    let obj = tobj::load_obj(file_name, &tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    });

    tobj::load_obj_buf(reader, load_options, material_loader)
}
