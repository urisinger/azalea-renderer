use std::sync::Arc;

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

#[derive(Debug, Resource)]
pub struct ChunkSender(pub flume::Sender<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>);

pub struct RenderPlugin {
    pub sender: flume::Sender<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>,
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
    mut query: Query<&mut InstanceHolder>,
    sender: bevy_ecs::system::Res<ChunkSender>,
) {
    for event in events.read() {
        let pos = ChunkPos::new(event.packet.x, event.packet.z);

        let local_player = query.get_mut(event.entity).unwrap();

        let instance = local_player.instance.read();

        if let Some(chunk) = instance.chunks.get(&pos) {
            sender.0.send((pos, chunk)).unwrap();
        } else {
            error!("Expected chunk, but none found");
        }
    }
}
