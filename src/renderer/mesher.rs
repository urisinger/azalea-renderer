use std::{
    array,
    borrow::Borrow,
    thread::{self, JoinHandle},
    time::Instant,
};

use azalea::{
    core::{
        direction::Direction,
        position::{ChunkPos, ChunkSectionBlockPos, ChunkSectionPos},
    },
    world::Section,
    BlockPos,
};
use glam::IVec3;

use crate::render_plugin::{offset_to_index, ChunkUpdate};

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
                let time = Instant::now();

                for y in 0..update.chunk.sections.len() {
                    let pos = ChunkSectionPos::new(update.pos.x, y as i32, update.pos.z);

                    let render_chunk = mesh_section(pos, &update);
                    section_send
                        .send(render_chunk)
                        .expect("Client disconnected, panicing.");
                }

                println!("Meshing chunk took: {}", time.elapsed().as_secs_f32());
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

pub fn mesh_section(pos: ChunkSectionPos, update: &ChunkUpdate) -> MeshUpdate {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Iterate over the chunk's blocks and generate mesh vertices
    for y in 0..16 {
        for x in 0..16 {
            for z in 0..16 {
                let pos = BlockPos::new(x, y + pos.y * 16, z);

                if !update.get_block(pos).is_some_and(|b| b.is_air()) {
                    for face in FACES {
                        indices.extend_from_slice(&[
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 1,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 3,
                        ]);
                        for offset in face.offsets {
                            let normal = face.dir.inormal();
                            let neighbor = BlockPos::new(x + normal.x, y + normal.y, z + normal.z);

                            if !update.get_block(neighbor).is_some_and(|b| b.is_air()) {
                                continue;
                            }

                            vertices.push(Vertex {
                                position: (offset + glam::IVec3::new(x as i32, y as i32, z as i32))
                                    .into(),
                            });
                        }
                    }
                }
            }
        }
    }
    MeshUpdate {
        pos,
        indices,
        vertices,
    }
}

struct Face {
    offsets: [IVec3; 4],
    dir: Direction,
}

const FACES: [Face; 6] = [
    Face {
        offsets: [
            glam::IVec3::new(0, 1, 0),
            glam::IVec3::new(0, 1, 1),
            glam::IVec3::new(1, 1, 1),
            glam::IVec3::new(1, 1, 0),
        ],
        dir: Direction::Up,
    },
    Face {
        offsets: [
            glam::IVec3::new(0, 0, 0),
            glam::IVec3::new(1, 0, 0),
            glam::IVec3::new(1, 0, 1),
            glam::IVec3::new(0, 0, 1),
        ],
        dir: Direction::Down,
    },
    Face {
        offsets: [
            glam::IVec3::new(0, 0, 1),
            glam::IVec3::new(1, 0, 1),
            glam::IVec3::new(1, 1, 1),
            glam::IVec3::new(0, 1, 1),
        ],
        dir: Direction::South,
    },
    Face {
        offsets: [
            glam::IVec3::new(0, 0, 0),
            glam::IVec3::new(0, 1, 0),
            glam::IVec3::new(1, 1, 0),
            glam::IVec3::new(1, 0, 0),
        ],
        dir: Direction::North,
    },
    Face {
        offsets: [
            glam::IVec3::new(1, 0, 0),
            glam::IVec3::new(1, 1, 0),
            glam::IVec3::new(1, 1, 1),
            glam::IVec3::new(1, 0, 1),
        ],
        dir: Direction::East,
    },
    Face {
        offsets: [
            glam::IVec3::new(0, 0, 0),
            glam::IVec3::new(0, 0, 1),
            glam::IVec3::new(0, 1, 1),
            glam::IVec3::new(0, 1, 0),
        ],
        dir: Direction::West,
    },
];
