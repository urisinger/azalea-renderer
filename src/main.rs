use azalea::core::position::ChunkPos;
use azalea::prelude::*;
use azalea::world::Chunk;
use azalea::{Account, ClientBuilder};
use bevy_ecs::component::Component;
use renderer::Renderer;
use std::sync::Arc;
use std::time::Instant;
use winit::window::CursorGrabMode;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

use crate::render_plguin::RenderPlugin;

mod render_plguin;
mod renderer;

async fn azlea_main(sender: flume::Sender<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>) {
    let account = Account::offline("bot");
    println!("hi from tokio");
    ClientBuilder::new()
        .set_handler(handle)
        .set_state(State::default())
        .add_plugins(RenderPlugin { sender })
        .start(account, "localhost:13157")
        .await
        .unwrap();
}

async fn handle(_bot: azalea::Client, event: azalea::Event, _state: State) -> anyhow::Result<()> {
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
    let (send, recv) = flume::unbounded();
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(azlea_main(send))
    });
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(main_render(recv));
}

async fn main_render(reciver: flume::Receiver<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = Renderer::new(&window, reciver).await;

    let mut last_render_time = Instant::now();
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
                }
                | WindowEvent::RedrawRequested => {
                    renderer.window().set_cursor_grab(CursorGrabMode::Confined).unwrap();
                    renderer.window().set_cursor_visible(false);
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
