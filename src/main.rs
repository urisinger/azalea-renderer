#![allow(dead_code)]
use azalea::pathfinder::goals::BlockPosGoal;
use azalea::{prelude::*, BlockPos};
use bevy_ecs::component::Component;
use render_plugin::ChunkUpdate;
use renderer::Renderer;
use std::time::Instant;
use winit::window::CursorGrabMode;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

use crate::render_plugin::RenderPlugin;
use log::*;

mod render_plugin;
mod renderer;

async fn azlea_main(
    main_updates: flume::Sender<ChunkUpdate>,
    neighbor_updates: flume::Sender<ChunkUpdate>,
) {
    let account = Account::offline("bot");

    ClientBuilder::new()
        .set_handler(handle)
        .set_state(State::default())
        .add_plugins(RenderPlugin {
            main_updates,
            neighbor_updates,
        })
        .start(account, "localhost:13157")
        .await
        .unwrap();
}

async fn handle(bot: azalea::Client, event: azalea::Event, _state: State) -> anyhow::Result<()> {
    match event {
        azalea::Event::Chat(m) => {
            println!("{}", m.message().to_ansi());
        }
        _ => {}
    }

    Ok(())
}

#[derive(Default, Clone, Component)]
pub struct State;

fn main() {
    let (main_sender, main_updates) = flume::unbounded();
    let (neighbor_sender, neighbor_updates) = flume::unbounded();

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(azlea_main(main_sender, neighbor_sender))
    });
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(main_render(main_updates, neighbor_updates));
}

async fn main_render(
    main_updates: flume::Receiver<ChunkUpdate>,
    neighbor_updates: flume::Receiver<ChunkUpdate>,
) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window, main_updates, neighbor_updates).await;

    let mut last_render_time = Instant::now();

    let mut is_locked = false;
    event_loop
        .run(move |event, elwt| match event {
            Event::AboutToWait => {
                renderer.window().request_redraw();
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => {
                renderer.camera_controller.process_mouse(delta.0, delta.1)
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == renderer.window().id() && !renderer.input(event) => match event {
                WindowEvent::CloseRequested => elwt.exit(),



                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    if is_locked{
                        let _ = renderer.window().set_cursor_grab(CursorGrabMode::None).inspect_err(|e| error!("Cannot free cursor: {}", e));
                        renderer.window().set_cursor_visible(true);

                    }else{
                        let _ = renderer.window().set_cursor_grab(CursorGrabMode::Confined).inspect_err(|e| error!("Cannot set confined, error: {}", e));
                        renderer.window().set_cursor_visible(false);
                    }
                    is_locked = !is_locked;

                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let dt = now - last_render_time;
                    last_render_time = now;
                    renderer.update(dt);
                    match renderer.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size()),
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                WindowEvent::Resized(size) => renderer.resize(*size),
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}
