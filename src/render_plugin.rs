use std::{array, time::Instant};

use azalea::{
    app::Plugin,
    chunks::ReceiveChunkEvent,
    core::{
        position::{ChunkPos, ChunkSectionBlockPos, ChunkSectionPos},
        tick::GameTick,
    },
    world::Chunk,
    BlockPos, InstanceHolder,
};
use bevy_ecs::{
    event::EventReader,
    schedule::IntoSystemConfigs,
    system::{Query, Resource},
};

use log::*;

#[derive(Debug)]
pub struct ChunkUpdate {
    pub pos: ChunkPos,
    pub chunk: Chunk,

    pub neighbers: [Option<Chunk>; 8],
}

impl ChunkUpdate {
    //BlockPos is relative to the chunk
    pub fn get_block(&self, pos: BlockPos, section_y: usize) -> Option<azalea::blocks::BlockState> {
        let chunk_pos = ChunkPos::from(pos);
        let y_offset = pos.y >> 4;

        let pos = ChunkSectionBlockPos::from(pos);
        if let Some(chunk_idx) = offset_to_index(chunk_pos) {
            self.neighbers[chunk_idx].as_ref().map(|c| {
                c.sections
                    .get((section_y as i32 + y_offset) as usize)
                    .map(|s| s.get(pos))
            })?
        } else {
            self.chunk
                .sections
                .get((section_y as i32 + y_offset) as usize)
                .map(|c| c.get(pos))
        }
    }
}

#[derive(Debug, Resource)]
pub struct ChunkSender {
    pub urgent_updates: flume::Sender<ChunkUpdate>,
}

pub struct RenderPlugin {
    pub sender: flume::Sender<ChunkUpdate>,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut azalea::app::App) {
        app.insert_resource(ChunkSender {
            urgent_updates: self.sender.clone(),
        })
        .add_systems(
            GameTick,
            send_chunks_system.after(azalea::chunks::handle_receive_chunk_events),
        );
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

fn send_chunks_system(
    mut events: EventReader<ReceiveChunkEvent>,
    query: Query<&InstanceHolder>,
    sender: bevy_ecs::system::Res<ChunkSender>,
) {
    for event in events.read() {
        let start = Instant::now();
        let pos = ChunkPos::new(event.packet.x, event.packet.z);

        let local_player = query.get(event.entity).unwrap();

        let instance = local_player.instance.read();

        let neighbers = array::from_fn(|i| {
            instance
                .chunks
                .get(&(pos + index_to_offset(i).expect("index should always be less then 8")))
                .map(|c| c.read().clone())
        });

        //Send non urgent update(update to neighbor chunks)
        neighbers
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.as_ref().map(|c| (i, c)))
            .for_each(|(i, c)| {
                let pos = pos + index_to_offset(i).expect("index should always be less then 0");
                let neighbers = array::from_fn(|i| {
                    instance
                        .chunks
                        .get(
                            &(pos
                                + index_to_offset(i).expect("index should always be less then 8")),
                        )
                        .map(|c| c.read().clone())
                });

                sender
                    .urgent_updates
                    .send(ChunkUpdate {
                        pos,
                        chunk: c.clone(),
                        neighbers,
                    })
                    .unwrap();
            });

        if let Some(chunk) = instance.chunks.get(&pos) {
            sender
                .urgent_updates
                .send(ChunkUpdate {
                    pos,
                    chunk: chunk.read().clone(),
                    neighbers,
                })
                .unwrap();
        } else {
            error!("Expected chunk, but none found");
        }

        println!("Sending chunk took: {}", start.elapsed().as_secs_f32());
    }
}
