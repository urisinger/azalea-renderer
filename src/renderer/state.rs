use winit::window::Window;

struct WindowData<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    pub window: &'a Window,
}

struct State<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    main_window: WindowData<'a>,
}
