use crate::prelude::{Mutex, Renderer, Ui};
use anyhow::anyhow;

pub async fn init(mut size: (u32, u32)) -> anyhow::Result<Renderer> {
    size.0 = size.0.max(1);
    size.1 = size.1.max(1);

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .ok_or_else(|| anyhow!("wgpu::Instance::request_adapter failed to find adapater"))?;

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

    let aspect_ratio = size.0 as f64 / size.1 as f64;

    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        label: None,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_view(&Default::default());

    let u32_size = std::mem::size_of::<u32>() as u32;
    let buffer_size = (u32_size * size.0 * size.1) as wgpu::BufferAddress;
    let buffer_desc = wgpu::BufferDescriptor {
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    };

    let buffer = device.create_buffer(&buffer_desc);

    let (ui, platform) = Ui::new_headless();
    Ok(Renderer {
        headless: true,
        instance,
        adapter,
        device,
        render_pass: Mutex::new(None),
        surface: None,
        queue,
        config: None,
        texture: Some(texture),
        texture_view: Some(texture_view),
        buffer: Some(buffer),
        platform,
        ui,
        aspect_ratio,
    })
}
