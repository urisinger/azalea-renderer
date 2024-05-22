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
