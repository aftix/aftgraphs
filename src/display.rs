use crate::{
    render::Renderer,
    ui::{Ui, UiWinitPlatform},
    GraphicsInitError,
};
use async_std::sync::Mutex;
use std::sync::Arc;
use wgpu;
use winit::window::Window;

pub async fn init(
    window: Arc<Window>,
) -> Result<Renderer<'static, UiWinitPlatform>, GraphicsInitError> {
    log::debug!("aftgraphs::display::init: Initializing display");

    let mut size = window.inner_size();
    // wgpu minimum surface size is 4x4
    size.width = size.width.max(4);
    size.height = size.height.max(4);

    log::debug!("aftgraphs::display::init: Creating surface");
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window.clone())?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .ok_or(GraphicsInitError::NoAdapter)?;

    log::debug!("aftgraphs::display::init: Requesting rendering device");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            },
            None,
        )
        .await?;

    log::debug!("aftgrahps::display::init: Adding wgpu error handler");
    fn unhandled_error(error: wgpu::Error) {
        log::error!("aftgraphs::display: wgpu unhandled error: {error:?}");
    }
    device.on_uncaptured_error(Box::new(unhandled_error));

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
        desired_maximum_frame_latency: 2,
    };

    log::debug!("aftgraphs::display::init: configuring surface");
    surface.configure(&device, &config);

    log::info!("surface configured");

    let aspect_ratio = size.width as f64 / size.height as f64;

    let (ui, platform) = Ui::new(&window, &device, &queue, swapchain_format);
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
        aspect_ratio,
        time: 0.0,
        delta_time: 0.0,
    })
}
