use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

pub static CUBE_DATA: &'static [f32] = &[
    // front
    -1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
     1.0, -1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
     1.0,  1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    -1.0,  1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    // back
    -1.0,  1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
     1.0,  1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
     1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0,
    // right
     1.0, -1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0,  1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0,  1.0,  1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
     1.0, -1.0,  1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
    // left
    -1.0, -1.0,  1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0,  1.0,  1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0,  1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
    // top
     1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    -1.0, 1.0,  1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
     1.0, 1.0,  1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
    // bottom
     1.0, -1.0,  1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
    -1.0, -1.0,  1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
    -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
     1.0, -1.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0,
];

pub static CUBE_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
    8, 9, 10, 10, 11, 8,
    12, 13, 14, 14, 15, 12,
    16, 17, 18, 18, 19, 16,
    20, 21, 22, 22, 23, 20,
];

pub static CUBE_VBL: &[VertexBufferLayout] = &[VertexBufferLayout{
    array_stride: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
    step_mode: VertexStepMode::Vertex,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x2,
            offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            shader_location: 1,
        },
        VertexAttribute {
            format: VertexFormat::Float32x3,
            offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
            shader_location: 2,
        },
    ],
}];

pub static FLOOR_DATA: &'static [f32] = &[
    -20.0, -1.0, -20.0, 0.0, 0.0,
    -20.0, -1.0,  20.0, 0.0, 1.0, 
     20.0, -1.0,  20.0, 1.0, 1.0,
     20.0, -1.0, -20.0, 1.0, 0.0,
];

pub static FLOOR_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
];