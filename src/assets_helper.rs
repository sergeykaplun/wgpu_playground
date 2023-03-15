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

pub async fn parse_data_url(data: &str) -> Result<Vec<u8>> {
    let sss: String = data.replace("data:application/octet-stream;base64,", "").trim().into();
    match base64::decode(sss) {
        Ok(data) => Ok(data),
        Err(error) => bail!("Failed to decode Base64 data"),
    }
    
    //Ok(decoded_data)

    // let media_type = parts[0].replace("data:", "");
    // let encoding_type: String = parts[1].replace("base64", "").replace("=", "").trim().into();

    // if encoding_type != "base64" {
    //     bail!("Invalid encoding type: {}", encoding_type);
    // }

    // let base64_data = parts.get(2).unwrap().trim();

    // let decoded_data = base64::decode(base64_data).ok().unwrap();

    // Ok(decoded_data)
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

pub async fn load_model_gltf(
    file_name: &str,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
) -> anyhow::Result<Model> {
    let gltf_text = load_string(file_name).await?;
    let gltf_cursor = Cursor::new(gltf_text);
    let gltf_reader = BufReader::new(gltf_cursor);
    let gltf = Gltf::from_reader(gltf_reader)?;

    // Load buffers
    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                    // if let Some(blob) = gltf.blob.as_deref() {
                //     buffer_data.push(blob.into());
                //     println!("Found a bin, saving");
                // };
            }
            gltf::buffer::Source::Uri(uri) => {
                //let bin = load_binary(uri).await?;
                //let decoded_data = base64::decode(base64_string).unwrap();
                let bin = parse_data_url(uri).await?;
                buffer_data.push(bin);
            }
        }
    }
    /*
    // Load animations
    let mut animation_clips = Vec::new();
    for animation in gltf.animations() {
        for channel in animation.channels() {
            let reader = channel.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let timestamps = if let Some(inputs) = reader.read_inputs() {
                match inputs {
                    gltf::accessor::Iter::Standard(times) => {
                        let times: Vec<f32> = times.collect();
                        println!("Time: {}", times.len());
                        dbg!(&times);
                        times
                    }
                    gltf::accessor::Iter::Sparse(_) => {
                        println!("Sparse keyframes not supported");
                        let times: Vec<f32> = Vec::new();
                        times
                    }
                }
            } else {
                println!("We got problems");
                let times: Vec<f32> = Vec::new();
                times
            };

            let keyframes = if let Some(outputs) = reader.read_outputs() {
                match outputs {
                    gltf::animation::util::ReadOutputs::Translations(translation) => {
                        let translation_vec = translation.map(|tr| {
                            // println!("Translation:");
                            dbg!(&tr);
                            let vector: Vec<f32> = tr.into();
                            vector
                        }).collect();
                        Keyframes::Translation(translation_vec)
                    },
                    other => {
                        Keyframes::Other
                    }
                    // gltf::animation::util::ReadOutputs::Rotations(_) => todo!(),
                    // gltf::animation::util::ReadOutputs::Scales(_) => todo!(),
                    // gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => todo!(),
                }
            } else {
                println!("We got problems");
                Keyframes::Other
            };

            animation_clips.push(AnimationClip {
                name: animation.name().unwrap_or("Default").to_string(),
                keyframes,
                timestamps,
            })
        }
    }
    */

    /*
    // Load materials
    let mut materials = Vec::new();
    for material in gltf.materials() {
        println!("Looping thru materials");
        let pbr = material.pbr_metallic_roughness();
        let base_color_texture = &pbr.base_color_texture();
        let texture_source = &pbr
            .base_color_texture()
            .map(|tex| {
                // println!("Grabbing diffuse tex");
                // dbg!(&tex.texture().source());
                tex.texture().source().source()
            })
            .expect("texture");

        match texture_source {
            gltf::image::Source::View { view, mime_type } => {
                let diffuse_texture = texture::Texture::from_bytes(
                    device,
                    queue,
                    &buffer_data[view.buffer().index()],
                    file_name,
                )
                .expect("Couldn't load diffuse");

                materials.push(model::Material {
                    name: material.name().unwrap_or("Default Material").to_string(),
                    diffuse_texture,
                });
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                let diffuse_texture = load_texture(uri, device, queue).await?;

                materials.push(model::Material {
                    name: material.name().unwrap_or("Default Material").to_string(),
                    diffuse_texture,
                });
            }
        };
    }
    */

    let mut meshes = Vec::new();

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            if let Some(mesh) = node.mesh() {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();

                mesh.primitives().for_each(|primitive| {
                    let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
                    if let Some(iter) = reader.read_positions() {
                        for (i, pos) in iter.enumerate() {
                            vertices.extend_from_slice(&pos);

                            if let Some(iter) = reader.read_tex_coords(0) {
                                vertices.extend_from_slice(&iter.into_f32().nth(i).unwrap());
                            }
                            if let Some(mut iter) = reader.read_normals() {
                                vertices.extend_from_slice(&iter.nth(i).unwrap());
                            }
                        }
                    }
                    
                    if let Some(iter) = reader.read_indices() {
                        for index in iter.into_u32() {
                            indices.push(index);
                        }
                    }
                });
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", file_name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", file_name)),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                meshes.push(Mesh {
                    name: file_name.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: indices.len() as u32,
                    // material: m.mesh.material_id.unwrap_or(0),
                    // material: 0,
                });
            }
            if let Some(mesh) = node.mesh() {
                
            }
        }
    }

    Ok(Model {
        meshes,
        // materials,
        // animations: animation_clips,
    })
}