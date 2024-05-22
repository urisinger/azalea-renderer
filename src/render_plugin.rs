use std::{array, sync::Arc, time::Instant};

use azalea::{
    app::Plugin,
    chunks::ReceiveChunkEvent,
    core::{position::ChunkPos, tick::GameTick},
    InstanceHolder,
};
use bevy_ecs::{
    event::EventReader,
    schedule::IntoSystemConfigs,
    system::{Query, Resource},
};

use parking_lot::RwLock;

#[derive(Debug)]
pub struct ChunkAdded {
    pub pos: ChunkPos,

    pub world: Arc<RwLock<azalea::world::Instance>>,
}

#[derive(Debug, Resource)]
pub struct ChunkSender {
    pub main_updates: flume::Sender<ChunkAdded>,
}

pub struct RenderPlugin {
    pub main_updates: flume::Sender<ChunkAdded>,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut azalea::app::App) {
        app.insert_resource(ChunkSender {
            main_updates: self.main_updates.clone(),
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
    sender: bevy_ecs::system::Res<ChunkSender>,

    query: Query<&mut InstanceHolder>,
) {
    for event in events.read() {
        let pos = ChunkPos::new(event.packet.x, event.packet.z);

        let local_player = query.get(event.entity).unwrap();

        sender
            .main_updates
            .send(ChunkAdded {
                pos,
                world: local_player.instance.clone(),
            })
            .unwrap();
    }
}
