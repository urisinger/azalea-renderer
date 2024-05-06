use std::{fs, io::BufReader, sync::Arc, time::Duration};

use azalea::{core::position::ChunkPos, world::Chunk};
use state::State;
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::PhysicalKey,
    window::{CursorGrabMode, Window},
};

use chunk::Vertex;
use log::*;

use self::{
    camera::{Camera, CameraController, Projection},
    chunk::RenderChunk,
    texture::Texture,
    world::{World, WorldRenderer},
};

mod camera;
mod chunk;
mod state;
mod texture;
mod world;

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.99240386], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.56958647], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.05060294], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.1526709], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.7347359], }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

pub struct Renderer<'a> {
    state: State<'a>,

    world: World,

    world_renderer: WorldRenderer,

    projection: Projection,
    camera: Camera,

    pub camera_controller: CameraController,

    reciver: flume::Receiver<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>,
}

impl<'a> Renderer<'a> {
    pub async fn new(
        window: &'a Window,
        reciver: flume::Receiver<(ChunkPos, Arc<parking_lot::RwLock<Chunk>>)>,
    ) -> Self {
        let state = State::new_async(window).await;

        let world = World::new();

        let world_renderer =
            WorldRenderer::new(&state.device, &state.queue, &state.main_window.config).unwrap();

        let camera_controller = CameraController::new(15.0, 0.5);

        let camera = Camera::new(
            (0.0, 5.0, 10.0),
            -90.0_f32.to_radians(),
            -20.0_f32.to_radians(),
        );

        let projection = Projection::new(
            state.main_window.config.width,
            state.main_window.config.height,
            45.0_f32.to_radians(),
            0.1,
            100.0,
        );

        Self {
            state,
            world,
            world_renderer,
            camera,
            projection,
            camera_controller,

            reciver,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.state.resize(new_size);
        self.world_renderer
            .resize(&self.state.device, &self.state.main_window.config);
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.state.main_window.size
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        while let Ok((pos, chunk)) = self.reciver.try_recv() {
            self.world_renderer
                .add_chunk(&self.state.device, &pos, &chunk.read());
        }
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.world_renderer
            .set_camera(self.projection.calc_matrix() * self.camera.calc_matrix());
        self.world_renderer.update(&self.state.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        println!("rendered");
        let output = self.state.main_window.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Main Render"),
                });

        self.world_renderer.draw(&mut encoder, &self.world, &view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.state.main_window.window
    }
}
