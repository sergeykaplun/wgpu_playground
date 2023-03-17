use std::io::{Cursor, BufReader};

use anyhow::{anyhow, bail, Result};
use gltf::Gltf;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new("./assets/").join(file_name);
    let txt = std::fs::read_to_string(path)?;
    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new("./assets/").join(file_name);
    let data = std::fs::read(path)?;
    Ok(data)
}

pub async fn parse_base64(data: &str) -> Result<Vec<u8>> {
    let sss: String = data.replace("data:application/octet-stream;base64,", "").trim().into();
    match base64::decode(sss) {
        Ok(data) => Ok(data),
        Err(error) => bail!("Failed to decode Base64 data"),
    }
}

pub async fn load_model(file_name: &str, device: &wgpu::Device) -> anyhow::Result<Vec<Mesh>> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, _obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .flat_map(|i|
                        vec!(m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2],
                            0.0, 0.0,
                            0.0, 0.0, 1.0
                            //m.mesh.normals[i * 3], m.mesh.normals[i * 3 + 1], m.mesh.normals[i * 3 + 2]
                        )
                )
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
            }
        })
        .collect::<Vec<_>>();
    Ok(meshes)
}