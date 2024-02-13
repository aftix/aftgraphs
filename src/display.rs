use crate::prelude::{Mutex, Renderer, Ui};
use anyhow::anyhow;
use winit::window::Window;

pub async fn init(window: &Window) -> anyhow::Result<Renderer> {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let instance = wgpu::Instance::default();
    let surface = unsafe {
        instance
            .create_surface(&window)
            .map_err(|err| anyhow!("wgpu::Instance::create_surface: {}", err))?
    };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .map_err(|err| anyhow!("wgpu::Adapter::request_device: {}", err))?;

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    let (ui, platform) = Ui::new(window, &device, &queue, swapchain_format);
    Ok(Renderer {
        headless: false,
        instance,
        adapter,
        device,
        render_pass: Mutex::new(None),
        surface: Some(surface),
        queue,
        config: Some(config),
        texture: None,
        texture_view: None,
        buffer: None,
        platform,
        ui,
    })
}
