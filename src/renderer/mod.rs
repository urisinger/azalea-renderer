use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

//pub mod assets;
//mod chunk;
mod mesher;
//mod world;

use bevy::{
    app::{App, Plugin, PluginGroup, Startup},
    asset::{io::AssetSourceId, AssetServer, Handle},
    ecs::system::{Commands, Res, Resource},
    log::LogPlugin,
    render::{camera::ClearColor, color::Color, texture::Image},
    tasks::{
        futures_lite::{FutureExt, StreamExt},
        AsyncComputeTaskPool, Task,
    },
    time::TimePlugin,
    utils::BoxedFuture,
    DefaultPlugins,
};

use self::mesher::ChunkMeshPlugin;

pub struct RenderPlugin {}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .disable::<TimePlugin>(),
        )
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(ChunkMeshPlugin);
    }
}

#[derive(Resource)]
pub struct TextureLoader(pub Task<Vec<Handle<Image>>>);

fn load_system(mut commands: Commands, asset_server: Res<AssetServer>) {
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
