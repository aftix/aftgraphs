use crate::render::Renderer;
use crate::ui::Ui;
use crate::GraphicsInitError;
use crate::{input::InputValue, simulation::InputEvent};
use async_mutex::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event at a certain time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HeadlessEvent {
    /// (position, button) - position is in [-1, 1], button is
    /// Left, Right, Middle, Back, Forward, Other(u16)
    MOUSEDOWN((f64, f64), winit::event::MouseButton),
    /// (position, button) - position is in [-1, 1], button is
    /// Left, Right, Middle, Back, Forward, Other(u16)
    MOUSEUP((f64, f64), winit::event::MouseButton),
    /// Keyboard event
    KEYEVENT(winit::event::RawKeyEvent),
}

impl From<HeadlessEvent> for InputEvent {
    fn from(value: HeadlessEvent) -> Self {
        use winit::event::ElementState;

        match value {
            HeadlessEvent::MOUSEDOWN(pos, button) => {
                Self::Mouse(ElementState::Pressed, button, pos)
            }
            HeadlessEvent::MOUSEUP(pos, button) => Self::Mouse(ElementState::Released, button, pos),
            HeadlessEvent::KEYEVENT(key_event) => Self::Keyboard(key_event),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HeadlessInputBlock {
    pub time: f64,
    #[serde(default)]
    pub events: Vec<HeadlessEvent>,
    #[serde(flatten)]
    pub inputs: HashMap<String, InputValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HeadlessMetadata {
    pub duration: f64,
    pub size: Option<[u32; 2]>,
    pub delta_t: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HeadlessInitialInputs {
    #[serde(flatten)]
    pub inputs: HashMap<String, InputValue>,
}

/// Input file for headless rendering
/// Input is in TOML
/// simulation TOML block defines total duration, size of render, and time step to use
/// Optional [initial-inputs] definies initial inputs
/// Each [[block]] defines a change in input at a specific time
/// Each input is the full input key from the spec file, with spaces
/// replaced by '_' and periods replaced by '-' (e.g. block_name-group_name-input_name)
/// as the key mapped to an InputValue
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HeadlessInput {
    pub simulation: HeadlessMetadata,
    #[serde(rename = "initial-inputs")]
    pub initial_inputs: Option<HeadlessInitialInputs>,
    #[serde(rename = "block", default)]
    pub blocks: Vec<HeadlessInputBlock>,
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn init(mut size: (u32, u32)) -> Result<Renderer<()>, GraphicsInitError> {
    use GraphicsInitError as HIE;

    log::debug!("aftgraphs::headless::init: Initializing renderer");

    size.0 = size.0.max(1);
    size.1 = size.1.max(1);

    log::debug!("aftgraphs::headless::init: Creating surface");
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .ok_or(HIE::NoAdapter)?;

    log::debug!("aftgraphs::headless::init: Requesting rendering device");
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
        .await?;

    log::debug!("aftgraphs::headless::init: Adding wgpu error handler");
    fn unhandled_error(error: wgpu::Error) {
        log::error!("aftgraphs::headless: wgpu unhandled error: {error:?}");
    }
    device.on_uncaptured_error(Box::new(unhandled_error));

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
    let bytes_per_row = u32_size * size.0;
    let missing_bytes =
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - (bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let bytes_per_row = bytes_per_row + missing_bytes;
    let buffer_size = (bytes_per_row * size.1) as wgpu::BufferAddress;
    let buffer_desc = wgpu::BufferDescriptor {
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("aftgraphs::headless"),
        mapped_at_creation: false,
    };

    let buffer = device.create_buffer(&buffer_desc);

    let (ui, platform) =
        Ui::new_headless(size, &device, &queue, wgpu::TextureFormat::Rgba8UnormSrgb);
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
