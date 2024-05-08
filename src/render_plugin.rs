use std::{array, sync::Arc};

use azalea::{
    app::Plugin,
    chunks::ReceiveChunkEvent,
    core::{position::ChunkPos, tick::GameTick},
    world::Chunk,
    InstanceHolder,
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
    pub chunk: Arc<parking_lot::RwLock<Chunk>>,

    pub neighbers: [Option<Arc<parking_lot::RwLock<Chunk>>>; 4],
}

#[derive(Debug, Resource)]
pub struct ChunkSender(pub flume::Sender<ChunkUpdate>);

pub struct RenderPlugin {
    pub sender: flume::Sender<ChunkUpdate>,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut azalea::app::App) {
        app.insert_resource(ChunkSender {
            0: self.sender.clone(),
        })
        .add_systems(
            GameTick,
            send_chunks_system.after(azalea::chunks::handle_receive_chunk_events),
        );
    }
}

fn send_chunks_system(
    mut events: EventReader<ReceiveChunkEvent>,
    query: Query<&InstanceHolder>,
    sender: bevy_ecs::system::Res<ChunkSender>,
) {
    for event in events.read() {
        let pos = ChunkPos::new(event.packet.x, event.packet.z);

        let local_player = query.get(event.entity).unwrap();

        let instance = local_player.instance.read();

        if let Some(chunk) = instance.chunks.get(&pos) {
            sender
                .0
                .send(ChunkUpdate {
                    pos,
                    chunk,
                    neighbers: array::from_fn(|i| {
                        const OFFSETS: [ChunkPos; 4] = [
                            ChunkPos { x: 0, z: -1 },
                            ChunkPos { x: 0, z: 1 },
                            ChunkPos { x: -1, z: 0 },
                            ChunkPos { x: 1, z: 0 },
                        ];

                        instance.chunks.get(&(pos + OFFSETS[i]))
                    }),
                })
                .unwrap();
        } else {
            error!("Expected chunk, but none found");
            panic!();
        }
    }
}
