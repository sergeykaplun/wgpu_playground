use std::io::{Cursor, BufReader};
use anyhow::bail;
use ktx::{include_ktx, Ktx, KtxInfo};
use wgpu::{Device, Extent3d, Queue, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView};
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

pub trait ResourceManager {
    fn load_string(&self, file_name: &str) -> anyhow::Result<String>;
    fn load_binary(&self, file_name: &str) -> anyhow::Result<Vec<u8>>;
    fn load_base64(&self, data: &str) -> anyhow::Result<Vec<u8>> {
        //let sss: String = data.replace("data:image/jpeg;base64,", "").trim().into();
        let sss = data.split("base64,").nth(1).unwrap().trim();
        match base64::decode(sss) {
            Ok(data) => Ok(data),
            Err(error) => bail!("Failed to decode Base64 data"),
        }
    }
    fn load_obj_model(&self, file_name: &str, device: &wgpu::Device) -> anyhow::Result<Vec<Mesh>>;
    fn empty_tex(&self, device: &Device, queue: &Queue) -> TextureView {
        self.load_tex_2d_ktx(device, queue,&include_ktx!("../assets/textures/papermill.ktx")).create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn load_tex_2d_ktx(&self, device: &Device, queue: &Queue, ktx_image: &Ktx<&[u8]>) -> Texture {
        //let ktx_image: Ktx<_> = include_ktx!(file_name);
        let format = TextureFormat::Rgba16Float;
        let texture_size = Extent3d {
            width: ktx_image.pixel_width(),
            height: ktx_image.pixel_height(),
            depth_or_array_layers: 1,
        };
        let empty_texture = device.create_texture(&TextureDescriptor {
            label: Some("Skybox Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let img_data: &[u8] = ktx_image.textures().nth(0).unwrap();
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &empty_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0, y: 0, z: 0,
                },
                aspect: TextureAspect::All,
            },
            img_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((8 * texture_size.width).into()),
                rows_per_image: Some(texture_size.height.into())
            },
            Extent3d {
                width: texture_size.width,
                height: texture_size.height,
                depth_or_array_layers: 1,
            },
        );
        empty_texture
    }
}

pub struct DesktopResourceManager;
impl ResourceManager for DesktopResourceManager {
    fn load_string(&self, file_name: &str) -> anyhow::Result<String> {
        let path = std::path::Path::new("./assets/").join(file_name);
        let txt = std::fs::read_to_string(path)?;
        Ok(txt)
    }

    fn load_binary(&self, file_name: &str) -> anyhow::Result<Vec<u8>> {
        let path = std::path::Path::new("./assets/").join(file_name);
        let data = std::fs::read(path)?;
        Ok(data)
    }

    fn load_obj_model(&self, file_name: &str, device: &wgpu::Device) -> anyhow::Result<Vec<Mesh>> {
        let obj_text = self.load_string(file_name)?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);
    
        let models/*(models, _obj_materials)*/ = pollster::block_on(
            tobj::load_obj_buf_async(
                &mut obj_reader,
                &tobj::LoadOptions {
                    triangulate: true,
                    single_index: true,
                    ..Default::default()
                },
                |p| async move {
                    let mat_text = self.load_string(&p).unwrap();
                    tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
                },
            )
        ).unwrap().0;
    
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
}

#[cfg(target_os = "android")]
pub mod android_resources {
    use anyhow::Ok;
    use ndk_sys::AAssetManager;
    use super::ResourceManager;

    pub struct AndroidResourceManager {
        pub asset_manager: *mut AAssetManager,
    }

    impl ResourceManager for AndroidResourceManager {
        fn load_string(&self, file_name: &str) -> anyhow::Result<String> {
            unsafe {
                let filename_cstr = std::ffi::CString::new(file_name).unwrap();
                let asset = ndk_sys::AAssetManager_open(self.asset_manager, filename_cstr.as_ptr(), ndk_sys::AASSET_MODE_UNKNOWN as i32);
                let buffer = ndk_sys::AAsset_getBuffer(asset);
                let length = ndk_sys::AAsset_getLength(asset);
                let data = std::slice::from_raw_parts(buffer as *const u8, length as usize).to_vec();
                ndk_sys::AAsset_close(asset);
                Ok(String::from_utf8(data).unwrap())
            }
        }

        fn load_binary(&self, file_name: &str) -> anyhow::Result<Vec<u8>> {
            unsafe {
                let filename_cstr = std::ffi::CString::new(file_name).unwrap();
                let asset = ndk_sys::AAssetManager_open(self.asset_manager, filename_cstr.as_ptr(), ndk_sys::AASSET_MODE_UNKNOWN as i32);
                let buffer = ndk_sys::AAsset_getBuffer(asset);
                let length = ndk_sys::AAsset_getLength(asset);
                let data = std::slice::from_raw_parts(buffer as *const u8, length as usize).to_vec();
                ndk_sys::AAsset_close(asset);
                Ok(data)
            }
        }

        fn load_obj_model(&self, _file_name: &str, _device: &wgpu::Device) -> anyhow::Result<Vec<super::Mesh>> {
            Ok(vec![])
        }
    }
}
