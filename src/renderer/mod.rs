use std::{sync::Arc, time::Duration};

use parking_lot::RwLock;
use state::State;
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::PhysicalKey,
    window::Window,
};

use crate::render_plugin::ChunkAdded;

use self::{
    assets::LoadedAssets,
    camera::{Camera, CameraController, Projection},
    mesher::Mesher,
    world::WorldRenderer,
};

pub mod assets;
mod camera;
mod chunk;
mod mesher;
mod state;
mod world;

pub struct Renderer<'a> {
    state: State<'a>,

    world_renderer: WorldRenderer,

    projection: Projection,
    camera: Camera,

    pub camera_controller: CameraController,

    mesher: Mesher,

    assets: Arc<LoadedAssets>,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: &'a Window, main_updates: flume::Receiver<ChunkAdded>) -> Self {
        let state = State::new_async(window).await;
        let assets = Arc::new(LoadedAssets::from_path(
            &state.device,
            &state.queue,
            env!("ASSETS_DIR"),
        ));

        let world_renderer =
            WorldRenderer::new(&state.device, &state.main_window.config, &assets).unwrap();

        let camera_controller = CameraController::new(15.0, 0.5);

        let camera = Camera::new((0.0, 128.0, 0.0), 0.0f32.to_radians(), 0.0f32.to_radians());

        let projection = Projection::new(
            state.main_window.config.width,
            state.main_window.config.height,
            45.0f32.to_radians(),
            0.1,
            100.0,
        );

        Self {
            state,
            world_renderer,
            camera,
            projection,
            camera_controller,

            mesher: Mesher::new(main_updates, assets.clone()),

            assets,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.state.resize(new_size);
        self.world_renderer
            .resize(&self.state.device, &self.state.main_window.config);
        self.projection
            .resize(new_size.width as f32, new_size.height as f32);
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
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.world_renderer
            .set_camera(self.projection.calc_matrix() * self.camera.calc_matrix());
        self.world_renderer.update(&self.state.queue);
        for update in self.mesher.iter() {
            self.world_renderer
                .update_chunk(&self.state.device, &update);

            println!("got chunk at pos: {:?}", update.pos);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

        self.world_renderer.draw(&mut encoder, &view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.state.main_window.window
    }
}
