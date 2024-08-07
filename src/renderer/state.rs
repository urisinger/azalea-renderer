use bevy_ecs::system::Resource;
use wgpu::PresentMode;
use winit::window::Window;

pub struct WindowData<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    pub window: &'a Window,
}

#[derive(Debug, Resource)]
pub struct State {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    //pub main_window: WindowData<'a>,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH
                        | wgpu::Features::POLYGON_MODE_LINE
                        | wgpu::Features::TEXTURE_BINDING_ARRAY
                        | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING ,
                    required_limits: adapter.limits(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_capability = surface.get_capabilities(&adapter);

        let surface_format = surface_capability
            .formats
            .iter()
            .find_map(|f| f.is_srgb().then_some(*f))
            .unwrap_or(surface_capability.formats[0]);

        let present_mode = surface_capability
            .present_modes
            .iter()
            .find_map(|p| (*p == PresentMode::Mailbox).then_some(*p))
            .unwrap_or(PresentMode::Fifo);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_capability.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            //main_window: WindowData {
            //    surface,
            //    config,
            //    size,
            //    window,
            //},
            instance,
            device,
            queue,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.main_window.size = new_size;
            self.main_window.config.width = new_size.width;
            self.main_window.config.height = new_size.height;
            self.main_window
                .surface
                .configure(&self.device, &self.main_window.config);
        }
    }
}
