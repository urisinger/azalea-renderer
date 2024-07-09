use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    },
};

//pub mod assets;
//mod chunk;
//mod mesher;
//mod world;

use azalea_client::{chunks::ReceiveChunkEvent, InstanceHolder};
use azalea_core::{position::ChunkPos, tick::GameTick};

use bevy::{
    app::{App, Plugin, PluginGroup, Startup},
    asset::{io::AssetSourceId, AssetServer, Handle},
    ecs::{
        event::EventReader,
        system::{Commands, Query, Res, Resource},
    },
    log::LogPlugin,
    render::texture::Image,
    tasks::{
        futures_lite::{FutureExt, StreamExt},
        AsyncComputeTaskPool, Task,
    },
    time::TimePlugin,
    utils::BoxedFuture,
    DefaultPlugins,
};

use parking_lot::RwLock;

use log::*;

#[derive(Debug)]
pub struct ChunkAdded {
    pub pos: ChunkPos,

    pub world: Arc<RwLock<azalea_world::Instance>>,
}

#[derive(Debug, Resource)]
pub struct ChunkSender {
    pub main_updates: flume::Sender<ChunkAdded>,
}

pub struct RenderPlugin {
    pub main_updates: flume::Sender<ChunkAdded>,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkSender {
            main_updates: self.main_updates.clone(),
        })
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .disable::<TimePlugin>(),
        )
        .add_systems(GameTick, send_chunks_system)
        .add_systems(Startup, load_assets_system);
    }
}

#[derive(Resource)]
pub struct TextureLoader(pub Task<Vec<Handle<Image>>>);

fn load_assets_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let thread_pool = AsyncComputeTaskPool::get();

    let server = asset_server.clone();
    commands.insert_resource(TextureLoader(thread_pool.spawn(async move {
        let mut images = Vec::new();
        load_assets_task(server, "minecraft/textures".into(), &mut images).await;
        images
    })));
}

fn load_assets_task<'a>(
    asset_server: AssetServer,
    path: PathBuf,
    images: &'a mut Vec<Handle<Image>>,
) -> BoxedFuture<'a, ()> {
    async move {
        let source = asset_server.get_source(AssetSourceId::Default).unwrap();
        let reader = source.reader();
        if let Ok(mut stream) = reader.read_directory(Path::new(&path)).await {
            while let Some(path) = stream.next().await {
                let is_directory = reader.is_directory(&path).await.unwrap_or(false);
                if is_directory {
                    load_assets_task(asset_server.clone(), path, images).await
                } else if path.extension().map(|e| e.to_str().unwrap()) == Some("png") {
                    let handle = asset_server.load::<Image>(path);
                    images.push(handle);
                }
            }
        }
    }
    .boxed()
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
            .main_updates
            .send(ChunkAdded {
                pos,
                world: local_player.instance.clone(),
            })
            .unwrap();
    }
}
