use std::{
    array,
    sync::Arc,
    thread::{self, JoinHandle},
    time::Instant,
};

use azalea::{
    blocks::Block,
    core::{
        direction::Direction,
        position::{ChunkBlockPos, ChunkPos, ChunkSectionBlockPos, ChunkSectionPos, Offset},
    },
    physics::collision::{BlockWithShape, Shapes},
    BlockPos,
};
use glam::IVec3;

use crate::render_plugin::ChunkAdded;

use super::{
    assets::{
        block_state::{BlockRenderState, Variant},
        model::Cube,
        LoadedAssets,
    },
    chunk::Vertex,
};

use log::*;

#[derive(Debug)]
pub struct ChunkLocal {
    pub chunk: azalea::world::Chunk,

    pub neighbers: [Option<azalea::world::Chunk>; 8],
}

impl ChunkLocal {
    //BlockPos is relative to the chunk
    pub fn get_block(&self, pos: BlockPos) -> Option<azalea::blocks::BlockState> {
        let chunk_pos = ChunkPos::from(pos);

        let pos = ChunkBlockPos::from(pos);

        if let Some(chunk_idx) = offset_to_index(chunk_pos) {
            self.neighbers[chunk_idx]
                .as_ref()
                .map(|c| c.get(&pos, -64))?
        } else {
            self.chunk.get(&pos, -64)
        }
    }
}

pub fn index_to_offset(index: usize) -> Option<ChunkPos> {
    match index {
        0 => Some(ChunkPos { x: 0, z: -1 }),  // North
        1 => Some(ChunkPos { x: 0, z: 1 }),   // South
        2 => Some(ChunkPos { x: 1, z: 0 }),   // East
        3 => Some(ChunkPos { x: -1, z: 0 }),  // West
        4 => Some(ChunkPos { x: 1, z: -1 }),  // Northeast
        5 => Some(ChunkPos { x: 1, z: 1 }),   // Southeast
        6 => Some(ChunkPos { x: -1, z: 1 }),  // Southwest
        7 => Some(ChunkPos { x: -1, z: -1 }), // Northwest
        _ => None,
    }
}

pub fn offset_to_index(offset: ChunkPos) -> Option<usize> {
    match offset {
        ChunkPos { x: 0, z: -1 } => Some(0),  // North
        ChunkPos { x: 0, z: 1 } => Some(1),   // South
        ChunkPos { x: 1, z: 0 } => Some(2),   // East
        ChunkPos { x: -1, z: 0 } => Some(3),  // West
        ChunkPos { x: 1, z: -1 } => Some(4),  // Northeast
        ChunkPos { x: 1, z: 1 } => Some(5),   // Southeast
        ChunkPos { x: -1, z: 1 } => Some(6),  // Southwest
        ChunkPos { x: -1, z: -1 } => Some(7), // Northwest
        _ => None,
    }
}

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
    pub fn new(main_updates: flume::Receiver<ChunkAdded>, assets: Arc<LoadedAssets>) -> Self {
        let (section_send, section_recv) = flume::unbounded();

        let chunk_thread = thread::spawn(move || loop {
            loop {
                for update in main_updates.iter() {
                    let time = Instant::now();

                    let local = {
                        let world = update.world.read();
                        ChunkLocal {
                            chunk: world.chunks.get(&update.pos).unwrap().read().clone(),
                            neighbers: array::from_fn(|i| {
                                world
                                    .chunks
                                    .get(
                                        &(update.pos
                                            + index_to_offset(i)
                                                .expect("index should always be less then 8")),
                                    )
                                    .map(|c| c.read().clone())
                            }),
                        }
                    };

                    for y in (-64..(320)).step_by(16) {
                        let pos = ChunkSectionPos::new(update.pos.x, y / 16 as i32, update.pos.z);

                        let render_chunk = mesh_section(pos, &local, &assets);
                        section_send
                            .send(render_chunk)
                            .expect("Client disconnected, panicing.");
                    }

                    info!("Meshing chunk took: {}", time.elapsed().as_secs_f32());
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
    update: &ChunkLocal,
    assets: &LoadedAssets,
) -> MeshUpdate {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Iterate over the chunk's blocks and generate mesh vertices
    for y in 0..16 {
        for x in 0..16 {
            for z in 0..16 {
                let block_pos = BlockPos::new(x, y + pos.y * 16, z);

                let block = update.get_block(block_pos).unwrap();

                let dyn_block = Box::<dyn Block>::from(block);

                let shape = block.shape();

                let block_state = assets.get_block_state(&format!("block/{}", dyn_block.id()));

                match block_state {
                    Some(BlockRenderState::Variants(variants)) => {
                        let variant = 'outer: {
                            let block_props = dyn_block.as_property_list();
                            for (states, variant) in variants {
                                let mut matched = true;
                                if states == "" {
                                    break 'outer variant;
                                }
                                for state in states.split(',') {
                                    let Some((name, value)) = state.split_once('=') else {
                                        error!(
                                            "could not find = in {}, states are: {:?}",
                                            state, states
                                        );
                                        continue;
                                    };
                                    let prop = block_props.get(name);

                                    if prop == Some(&value.to_string()) {
                                        continue;
                                    } else if prop == None {
                                        error!("could not find prop {} in {:?}", name, block_props);
                                    } else {
                                        matched = false;
                                        continue;
                                    };
                                }
                                if matched {
                                    break 'outer variant;
                                }
                            }

                            &variants[0].1
                        };

                        let desc = match variant {
                            Variant::Single(desc) => desc,
                            Variant::Array(desc) => &desc[0],
                        };

                        let model = assets.get_block_model(&desc.model);

                        match model {
                            Some(model) => match model.elements() {
                                Some(elements) => {
                                    for element in elements {
                                        for face in FACES {
                                            let model_face = match face.dir {
                                                Direction::Down => &element.faces.down,
                                                Direction::Up => &element.faces.up,
                                                Direction::North => &element.faces.north,
                                                Direction::South => &element.faces.south,
                                                Direction::West => &element.faces.west,
                                                Direction::East => &element.faces.east,
                                            };

                                            match model_face {
                                                Some(model_face) => {
                                                    let len = vertices.len() as u16;

                                                    let normal = face.dir.inormal();

                                                    let cull_face = model_face
                                                        .cullface
                                                        .as_deref()
                                                        .map(|s| match s {
                                                            "down" => Some(Direction::Down),
                                                            "up" => Some(Direction::Up),
                                                            "north" => Some(Direction::North),
                                                            "south" => Some(Direction::South),
                                                            "west" => Some(Direction::West),
                                                            "east" => Some(Direction::East),
                                                            _ => {
                                                                error!("Could not find cullface, make sure the assets folder is ok");
                                                                None
                                                            },
                                                        }).flatten();

                                                    if cull_face.is_some_and(|cull_face| {
                                                        let cull_normal = cull_face.inormal();

                                                        let cull_neighbor = BlockPos::new(
                                                            block_pos.x + cull_normal.x,
                                                            block_pos.y + cull_normal.y,
                                                            block_pos.z + cull_normal.z,
                                                        );

                                                        update.get_block(cull_neighbor).is_some_and(
                                                            |b| {
                                                                !b.is_air()
                                                                    && !Shapes::matches_anywhere(
                                                                        &shape,
                                                                        &b.shape(),
                                                                        |b1, b2| b1 && !b2,
                                                                    )
                                                            },
                                                        )
                                                    }) {
                                                        continue;
                                                    }

                                                    let uvs = generate_uv(face.dir, model_face.uv);
                                                    for (i, offset) in
                                                        face.offsets.iter().enumerate()
                                                    {
                                                        let tex_idx = model
                                                            .get_texture(&model_face.texture)
                                                            .map(|name| {
                                                                let tex_idx =
                                                                    assets.get_texture_id(&name);

                                                                if tex_idx.is_none(){
                                                                    error!("failed getting texture index for {}", name)
                                                                }

                                                                tex_idx
                                                            })
                                                            .flatten();

                                                        vertices.push(Vertex {
                                                            position: (offset_to_coord(
                                                                *offset, element,
                                                            ) / 16.0
                                                                + glam::Vec3::new(
                                                                    x as f32, y as f32, z as f32,
                                                                ))
                                                            .into(),
                                                            ao: if model.ambient_occlusion {
                                                                compute_ao(
                                                                    block_pos, *offset, normal,
                                                                    update,
                                                                )
                                                            } else {
                                                                3
                                                            },
                                                            tex_idx: tex_idx.unwrap_or(0) as u32,
                                                            uv: uvs[i].into(),
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
                                                None => {}
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            None => error!("could not get model: {}", desc.model),
                        }
                    }
                    Some(BlockRenderState::MultiPart) => {}
                    None => error!("Block state does not exist for block {}", dyn_block.id()),
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

fn generate_uv(dir: Direction, uvs: Option<[f32; 4]>) -> [glam::Vec2; 4] {
    match uvs {
        Some(uvs) => match dir {
            Direction::Up => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
            ],
            Direction::Down => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
            ],
            Direction::North => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
            ],
            Direction::South => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
            ],
            Direction::East => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
            ],
            Direction::West => [
                glam::Vec2::new(uvs[0] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[3] / 16.0),
                glam::Vec2::new(uvs[2] / 16.0, uvs[1] / 16.0),
                glam::Vec2::new(uvs[0] / 16.0, uvs[1] / 16.0),
            ],
        },
        None => match dir {
            Direction::Up => [
                glam::Vec2::new(0.0, 0.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(1.0, 1.0),
                glam::Vec2::new(0.0, 1.0),
            ],
            Direction::Down => [
                glam::Vec2::new(0.0, 0.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(1.0, 1.0),
                glam::Vec2::new(0.0, 1.0),
            ],
            Direction::North => [
                glam::Vec2::new(0.0, 1.0),
                glam::Vec2::new(0.0, 0.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(1.0, 1.0),
            ],
            Direction::South => [
                glam::Vec2::new(0.0, 1.0),
                glam::Vec2::new(1.0, 1.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(0.0, 0.0),
            ],
            Direction::East => [
                glam::Vec2::new(0.0, 1.0),
                glam::Vec2::new(0.0, 0.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(1.0, 1.0),
            ],
            Direction::West => [
                glam::Vec2::new(0.0, 1.0),
                glam::Vec2::new(1.0, 1.0),
                glam::Vec2::new(1.0, 0.0),
                glam::Vec2::new(0.0, 0.0),
            ],
        },
    }
}

fn offset_to_coord(offset: IVec3, element: &Cube) -> glam::Vec3 {
    glam::Vec3::new(
        if offset.x == 0 {
            element.from.x
        } else {
            element.to.x
        },
        if offset.y == 0 {
            element.from.y
        } else {
            element.to.y
        },
        if offset.z == 0 {
            element.from.z
        } else {
            element.to.z
        },
    )
}

fn compute_ao(pos: BlockPos, offset: glam::IVec3, face_normal: Offset, update: &ChunkLocal) -> u32 {
    let ao = if face_normal.x != 0 {
        let side1 = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, 0, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, 0))
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        ao(side1, side2, corner)
    } else if face_normal.y != 0 {
        let side1 = update
            .get_block(pos + BlockPos::new(0, offset.y * 2 - 1, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, 0))
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        ao(side1, side2, corner)
    } else {
        let side1 = update
            .get_block(pos + BlockPos::new(0, offset.y * 2 - 1, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        let side2 = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, 0, offset.z * 2 - 1))
            .is_some_and(|b| b.is_shape_full());

        let corner = update
            .get_block(pos + BlockPos::new(offset.x * 2 - 1, offset.y * 2 - 1, offset.z * 2 - 1))
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
