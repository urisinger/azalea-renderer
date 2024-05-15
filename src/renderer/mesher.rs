use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Instant,
};

use azalea::{
    core::{
        direction::Direction,
        position::{ChunkSectionPos, Offset},
    },
    physics::collision::BlockWithShape,
    BlockPos,
};
use glam::IVec3;

use crate::render_plugin::ChunkUpdate;

use super::{assets::LoadedAssets, chunk::Vertex};

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
    pub fn new(
        main_updates: flume::Receiver<ChunkUpdate>,
        neighbor_updates: flume::Receiver<ChunkUpdate>,
        assets: Arc<LoadedAssets>,
    ) -> Self {
        let (section_send, section_recv) = flume::unbounded();

        let chunk_thread = thread::spawn(move || loop {
            loop {
                for update in main_updates.try_iter() {
                    let time = Instant::now();

                    for y in 0..update.chunk.sections.len() {
                        let pos = ChunkSectionPos::new(update.pos.x, y as i32, update.pos.z);

                        let render_chunk = mesh_section(pos, &update, assets.clone());
                        section_send
                            .send(render_chunk)
                            .expect("Client disconnected, panicing.");
                    }

                    println!("Meshing chunk took: {}", time.elapsed().as_secs_f32());
                }

                for update in neighbor_updates.try_iter() {
                    for y in 0..update.chunk.sections.len() {
                        let pos = ChunkSectionPos::new(update.pos.x, y as i32, update.pos.z);

                        let render_chunk = mesh_section(pos, &update, assets.clone());
                        section_send
                            .send(render_chunk)
                            .expect("Client disconnected, panicing.");
                    }

                    if !main_updates.is_empty() {
                        break;
                    }
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
    update: &ChunkUpdate,
    assets: Arc<LoadedAssets>,
) -> MeshUpdate {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Iterate over the chunk's blocks and generate mesh vertices
    for y in 0..16 {
        for x in 0..16 {
            for z in 0..16 {
                let block_pos = BlockPos::new(x, y, z);

                let block = update.get_block(block_pos, pos.y as usize).unwrap();

                if !block.is_air() {
                    for face in FACES {
                        let len = vertices.len() as u16;

                        let normal = face.dir.inormal();
                        let neighbor = BlockPos::new(x + normal.x, y + normal.y, z + normal.z);

                        if update
                            .get_block(neighbor, pos.y as usize)
                            .is_some_and(|b| !b.is_air())
                        {
                            continue;
                        }

                        for offset in face.offsets {
                            vertices.push(Vertex {
                                position: (offset + glam::IVec3::new(x as i32, y as i32, z as i32))
                                    .into(),
                                ao: compute_ao(block_pos, pos.y as usize, offset, normal, update),
                            });
                        }
                        indices.extend_from_slice(&[
                            len + 0,
                            len + 1,
                            len + 2,
                            len + 0,
                            len + 2,
                            len + 3,
                        ]);
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

fn compute_ao(
    pos: BlockPos,
    section_y: usize,
    offset: glam::IVec3,
    face_normal: Offset,
    update: &ChunkUpdate,
) -> u32 {
    let ao = if face_normal.x != 0 {
        let side1 = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, 0, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, 0),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        ao(side1, side2, corner)
    } else if face_normal.y != 0 {
        let side1 = update
            .get_block(
                pos + BlockPos::new(0, offset.y * 2 - 1, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, 0),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        ao(side1, side2, corner)
    } else {
        let side1 = update
            .get_block(
                pos + BlockPos::new(0, offset.y * 2 - 1, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, 0, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(
                pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1),
                section_y,
            )
            .is_some_and(|b| b.is_shape_full());

        ao(side1, side2, corner)
    };

    ao
}

fn ao(side1: bool, side2: bool, corner: bool) -> u32 {
    if side1 && side2 {
        0
    } else {
        3 - ((side1 || side2) as u32 + corner as u32)
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
