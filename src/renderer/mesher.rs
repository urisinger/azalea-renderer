use std::{array, sync::Arc, time::Instant};

use azalea_client::{chunks::ReceiveChunkEvent, InstanceHolder};
use azalea_core::{
    direction::Direction,
    position::{BlockPos, ChunkBlockPos, ChunkPos, ChunkSectionPos},
    tick::GameTick,
};
use azalea_physics::collision::BlockWithShape;
use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    tasks::AsyncComputeTaskPool,
};
use bevy_flycam::FlyCam;
use glam::IVec3;
use parking_lot::RwLock;

#[derive(Debug)]
pub struct ChunkLocal {
    pub chunk: azalea_world::Chunk,

    pub neighbers: [Option<azalea_world::Chunk>; 8],
}

impl ChunkLocal {
    //BlockPos is relative to the chunk
    pub fn get_block(&self, pos: BlockPos) -> Option<azalea_block::BlockState> {
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

#[derive(Debug)]
pub struct ChunkAdded {
    pub pos: ChunkPos,

    pub world: Arc<RwLock<azalea_world::Instance>>,
}

#[derive(Debug, Resource)]
pub struct ChunkSender {
    pub chunks_send: flume::Sender<ChunkAdded>,
}

#[derive(Debug, Resource)]
pub struct MeshReciver {
    pub mesh_recv: flume::Receiver<(Transform, Mesh)>,
}

pub struct ChunkMeshPlugin;

impl Plugin for ChunkMeshPlugin {
    fn build(&self, app: &mut App) {
        let (chunks_send, chunks_recv) = flume::unbounded();
        let (mesh_send, mesh_recv) = flume::unbounded();
        app.add_systems(GameTick, send_chunks_system)
            .add_systems(Update, (insert_mesh_system, test_system))
            .insert_resource(MeshReciver { mesh_recv })
            .insert_resource(ChunkSender { chunks_send });

        let thread_pool = AsyncComputeTaskPool::get();
        thread_pool
            .spawn(create_meshes_task(chunks_recv, mesh_send))
            .detach();
    }
}

fn send_chunks_system(
    mut events: EventReader<ReceiveChunkEvent>,
    sender: Res<ChunkSender>,

    query: Query<&InstanceHolder>,
) {
    for event in events.read() {
        let pos = ChunkPos::new(event.packet.x, event.packet.z);

        let local_player = query.get(event.entity).unwrap();

        sender
            .chunks_send
            .send(ChunkAdded {
                pos,
                world: local_player.instance.clone(),
            })
            .unwrap();
    }
}

fn insert_mesh_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    recv_meshes: Res<MeshReciver>,
) {
    for (transform, mesh) in recv_meshes.mesh_recv.try_iter() {
        commands.spawn(MaterialMeshBundle {
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial::from(Color::WHITE)),
            transform,
            ..Default::default()
        });
    }
}

fn test_system(cameras: Query<(&FlyCam, &mut Transform)>) {
    for (camera, transform) in &cameras {
        println!("{:?}", transform);
    }
}
async fn create_meshes_task(
    chunks_recv: flume::Receiver<ChunkAdded>,
    mesh_send: flume::Sender<(Transform, Mesh)>,
) {
    while let Ok(update) = chunks_recv.recv_async().await {
        let time = Instant::now();

        let local = {
            let world = update.world.read();
            let chunk = if let Some(chunk) = world.chunks.get(&update.pos) {
                chunk.read().clone()
            } else {
                error!("could not find chunk");
                continue;
            };

            ChunkLocal {
                chunk,
                neighbers: array::from_fn(|i| {
                    world
                        .chunks
                        .get(
                            &(update.pos
                                + index_to_offset(i).expect("index should always be less then 8")),
                        )
                        .map(|c| c.read().clone())
                }),
            }
        };

        for y in (-64..(320)).step_by(16) {
            let pos = ChunkSectionPos::new(update.pos.x, y / 16 as i32, update.pos.z);

            let render_chunk = mesh_section(pos, &local);
            mesh_send
                .send((
                    Transform::from_xyz(
                        (pos.x * 16) as f32,
                        (pos.y * 16) as f32,
                        (pos.z * 16) as f32,
                    ),
                    render_chunk,
                ))
                .expect("Client disconnected, panicing.");
        }

        info!(
            "Meshing chunk took: {}, with pos: {:?}",
            time.elapsed().as_secs_f32(),
            update.pos
        );
    }
}

pub fn mesh_section(pos: ChunkSectionPos, update: &ChunkLocal) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices = Vec::new();

    // Iterate over the chunk's blocks and generate mesh vertices
    //
    for y in 0..16 {
        for x in 0..16 {
            for z in 0..16 {
                let block_pos = BlockPos::new(x, y + pos.y * 16, z);

                if !update.get_block(block_pos).is_some_and(|b| b.is_air()) {
                    for face in FACES {
                        indices.extend_from_slice(&[
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 1,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 0,
                            vertices.len() as u16 + 2,
                            vertices.len() as u16 + 3,
                        ]);
                        let normal = face.dir.normal();
                        let normal = IVec3::new(
                            normal.x.round() as i32,
                            normal.y.round() as i32,
                            normal.z.round() as i32,
                        );

                        for offset in face.offsets {
                            let neighbor = BlockPos::new(x + normal.x, y + normal.y, z + normal.z);

                            if !update.get_block(neighbor).is_some_and(|b| b.is_air()) {
                                continue;
                            }

                            vertices.push(
                                (offset + glam::IVec3::new(x as i32, y as i32, z as i32))
                                    .as_vec3()
                                    .into(),
                            );
                        }
                    }
                }
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_indices(Indices::U16(indices));

    mesh
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

fn compute_ao(
    pos: BlockPos,
    offset: glam::IVec3,
    face_normal: glam::IVec3,
    update: &ChunkLocal,
) -> u32 {
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
