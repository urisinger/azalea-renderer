use std::{num::NonZeroU64, ops::Add};

use azalea::{core::position::ChunkSectionBlockPos, world::Section};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct RenderChunk {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub bind_group: wgpu::BindGroup,
    pub len: u32,
}

const FACES: [[Vertex; 4]; 6] = [
    //top(+y)
    [
        Vertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ],
    //bottom(-y)
    [
        Vertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ],
    //forward(+z)
    [
        Vertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
        },
    ],
    //backwards(-z)
    [
        Vertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ],
    //right(+x)
    [
        Vertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
        },
    ],
    //left(-x)
    [
        Vertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
        },
    ],
];

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkUniform {
    pos: [i32; 3],
}

impl RenderChunk {
    pub fn from_section(
        device: &wgpu::Device,

        layout: &wgpu::BindGroupLayout,
        section: &Section,
        pos: glam::IVec3,
    ) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Iterate over the chunk's blocks and generate mesh vertices
        for y in 0..16 {
            for x in 0..16 {
                for z in 0..16 {
                    if !section.get(ChunkSectionBlockPos::new(x, y, z)).is_air() {
                        for face in FACES {
                            for vertex in face {
                                vertices.push(Vertex {
                                    position: glam::Vec3::from_array(vertex.position)
                                        .add(glam::Vec3::new(x as f32, y as f32, z as f32))
                                        .into(),
                                    tex_coords: vertex.tex_coords,
                                })
                            }

                            indices.extend_from_slice(&[
                                vertices.len() as u16 + 0,
                                vertices.len() as u16 + 1,
                                vertices.len() as u16 + 2,
                                vertices.len() as u16 + 0,
                                vertices.len() as u16 + 2,
                                vertices.len() as u16 + 3,
                            ])
                        }
                    }
                }
            }
        }

        Self::from_vertex_index(device, layout, &vertices, &indices, pos)
    }

    pub fn from_vertex_index(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        vertecies: &[Vertex],
        indicies: &[u16],
        pos: glam::IVec3,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertecies),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indicies),
            usage: wgpu::BufferUsages::INDEX,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[ChunkUniform { pos: pos.into() }]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: NonZeroU64::new(uniform_buffer.size()),
                }),
            }],
            label: None,
            layout,
        });

        Self {
            vertex_buffer,
            index_buffer,
            bind_group,
            len: indicies.len() as u32,
        }
    }
}
