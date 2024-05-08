use std::{
    array,
    thread::{self, JoinHandle},
};

use azalea::{
    core::{
        direction::Direction,
        position::{ChunkSectionBlockPos, ChunkSectionPos},
    },
    world::Section,
};

use crate::render_plugin::ChunkUpdate;

use super::chunk::Vertex;

pub struct MeshUpdate {
    pub pos: ChunkSectionPos,

    pub indices: Vec<u16>,
    pub vertices: Vec<Vertex>,
}

pub struct Mesher {
    chunk_thread: JoinHandle<()>,

    section_recv: flume::Receiver<MeshUpdate>,
}

impl Mesher {
    pub fn new(reciver: flume::Receiver<ChunkUpdate>) -> Self {
        let (section_send, section_recv) = flume::unbounded();

        let chunk_thread = thread::spawn(move || {
            for update in reciver.iter() {
                let mut y = 0;
                let chunk = update.chunk.read();
                for section in &chunk.sections {
                    let pos = ChunkSectionPos::new(update.pos.x, y, update.pos.z);
                    let neighbers = array::from_fn(|i| {
                        let dir: Direction = unsafe { std::mem::transmute(i as u8) };

                        match dir {
                            Direction::Down => {
                                if y as i32 >= 1 {
                                    chunk.sections.get((y - 1) as usize).cloned()
                                } else {
                                    None
                                }
                            }
                            Direction::Up => chunk.sections.get(y as usize + 1).cloned(),
                            Direction::North => update.neighbers[0]
                                .clone()
                                .map(|c| c.read().sections.get(y as usize).cloned())
                                .flatten(),
                            Direction::South => update.neighbers[1]
                                .clone()
                                .map(|c| c.read().sections.get(y as usize).cloned())
                                .flatten(),
                            Direction::West => update.neighbers[2]
                                .clone()
                                .map(|c| c.read().sections.get(y as usize).cloned())
                                .flatten(),
                            Direction::East => update.neighbers[3]
                                .clone()
                                .map(|c| c.read().sections.get(y as usize).cloned())
                                .flatten(),
                        }
                    });

                    let render_chunk = mesh_section(pos, section, &neighbers);
                    section_send
                        .send(render_chunk)
                        .expect("Client disconnected, panicing.");
                    y += 1;
                }
            }
        });

        Self {
            section_recv,
            chunk_thread,
        }
    }

    pub fn iter(&self) -> flume::TryIter<MeshUpdate> {
        self.section_recv.try_iter()
    }
}

pub fn mesh_section(
    pos: ChunkSectionPos,
    section: &Section,
    neighbers: &[Option<Section>; 6],
) -> MeshUpdate {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Iterate over the chunk's blocks and generate mesh vertices
    for y in 0..16 {
        for x in 0..16 {
            for z in 0..16 {
                let pos = ChunkSectionBlockPos::new(x, y, z);
                if !section.get(pos).is_air() {
                    for face in FACES {
                        indices.extend_from_slice(&[
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 1,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 3,
                        ]);
                        for vertex in face.vertices {
                            let normal = face.dir.inormal();
                            let neighbor = ChunkSectionBlockPos::new(
                                (x as i8 + normal.x as i8).max(0) as u8,
                                (y as i8 + normal.y as i8).max(0) as u8,
                                (z as i8 + normal.z as i8).max(0) as u8,
                            );

                            if neighbor.x < 16
                                && neighbor.y < 16
                                && neighbor.z < 16
                                && x as i8 + normal.x as i8 >= 0
                                && y as i8 + normal.y as i8 >= 0
                                && z as i8 + normal.z as i8 >= 0
                            {
                                if !section.get(neighbor).is_air() {
                                    continue;
                                }
                            } else {
                                if let Some(section) = &neighbers[unsafe {
                                    std::mem::transmute::<Direction, u8>(face.dir) as usize
                                }] {
                                    let new_chunk_pos = match face.dir {
                                        Direction::Down => {
                                            ChunkSectionBlockPos::new(pos.x, 15, pos.z)
                                        }
                                        Direction::Up => ChunkSectionBlockPos::new(pos.x, 0, pos.z),
                                        Direction::North => {
                                            ChunkSectionBlockPos::new(pos.x, pos.y, 15)
                                        }
                                        Direction::South => {
                                            ChunkSectionBlockPos::new(pos.x, pos.y, 0)
                                        }
                                        Direction::West => {
                                            ChunkSectionBlockPos::new(15, pos.y, pos.z)
                                        }
                                        Direction::East => {
                                            ChunkSectionBlockPos::new(0, pos.y, pos.z)
                                        }
                                    };

                                    if !section.get(new_chunk_pos).is_air() {
                                        continue;
                                    }
                                }
                            }

                            vertices.push(Vertex {
                                position: (glam::Vec3::from_array(vertex.position)
                                    + glam::Vec3::new(x as f32, y as f32, z as f32))
                                .into(),
                                tex_coords: vertex.tex_coords,
                            });
                        }
                    }
                }
            }
        }
    }
    MeshUpdate {
        pos: pos,
        indices,
        vertices,
    }
}

struct Face {
    vertices: [Vertex; 4],
    dir: Direction,
}

const FACES: [Face; 6] = [
    Face {
        vertices: [
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
        dir: Direction::Up,
    },
    Face {
        vertices: [
            Vertex {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0],
                tex_coords: [0.0, 1.0],
            },
        ],
        dir: Direction::Down,
    },
    Face {
        vertices: [
            Vertex {
                position: [0.0, 0.0, 1.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, 0.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0],
                tex_coords: [0.0, 1.0],
            },
        ],
        dir: Direction::South,
    },
    Face {
        vertices: [
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
        dir: Direction::North,
    },
    Face {
        vertices: [
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
        dir: Direction::East,
    },
    Face {
        vertices: [
            Vertex {
                position: [0.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.0, 0.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [0.0, 1.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
        ],
        dir: Direction::West,
    },
];
