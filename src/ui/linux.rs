use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::{ClipboardBackend, Context, FontConfig, FontSource};
use imgui_wgpu::{Renderer as ImguiRenderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use wgpu::{Device, Queue, TextureFormat};
use winit::{event::Event, window::Window};

pub trait UiPlatform {
    fn prepare_frame(&mut self, ui: &mut Ui, window: &Window);
    fn prepare_render(&mut self, frame: &mut imgui::Ui, window: &Window);
    fn handle_event<T>(&mut self, ui: &mut Ui, window: &Window, event: &Event<T>);
}

pub struct UiWinitPlatform(WinitPlatform);

impl UiPlatform for UiWinitPlatform {
    fn prepare_frame(&mut self, ui: &mut Ui, window: &Window) {
        self.0
            .prepare_frame(ui.0.io_mut(), window)
            .expect("aftgraphs::ui::UiPlatform::prepare_frame: Enexpected failure");
    }

    fn prepare_render(&mut self, frame: &mut imgui::Ui, window: &Window) {
        self.0.prepare_render(frame, window);
    }

    fn handle_event<T>(&mut self, ui: &mut Ui, window: &Window, event: &Event<T>) {
        self.0.handle_event(ui.0.io_mut(), window, event);
    }
}

impl UiPlatform for () {
    fn prepare_frame(&mut self, _ui: &mut Ui, _window: &Window) {
        panic!("Do not call any platform functions on headless")
    }

    fn prepare_render(&mut self, _frame: &mut imgui::Ui, _window: &Window) {
        panic!("Do not call any platform functions on headless")
    }

    fn handle_event<T>(&mut self, _ui: &mut Ui, _window: &Window, _event: &Event<T>) {
        panic!("Do not call any platform functions on headless")
    }
}

struct ClipboardSupport(ClipboardContext);

impl ClipboardSupport {
    pub fn new() -> Option<Self> {
        ClipboardContext::new().ok().map(ClipboardSupport)
    }
}

impl ClipboardBackend for ClipboardSupport {
    fn get(&mut self) -> Option<String> {
        self.0.get_contents().ok()
    }

    fn set(&mut self, text: &str) {
        self.0.set_contents(text.to_owned()).ok();
    }
}

pub struct Ui(Context, ImguiRenderer);

impl Ui {
    pub(crate) fn context_mut(&mut self) -> &mut Context {
        &mut self.0
    }

    pub fn draw<'a, 'b: 'a>(
        &'b mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) -> Result<(), imgui_wgpu::RendererError> {
        self.1.render(self.0.render(), queue, device, render_pass)
    }

    pub fn new(
        window: &Window,
        device: &Device,
        queue: &Queue,
        swapchain_format: TextureFormat,
    ) -> (Self, UiWinitPlatform) {
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut ctx);
        {
            let dpi_mode = if let Ok(factor) = std::env::var("IMGUI_FORCE_DPI_FACTOR") {
                match factor.parse::<f64>() {
                    Ok(f) => HiDpiMode::Locked(f),
                    Err(e) => panic!("Invalid winit scaling factor: {}", e),
                }
            } else {
                HiDpiMode::Default
            };

            platform.attach_window(ctx.io_mut(), window, dpi_mode);
        }

        if let Some(clipboard) = ClipboardSupport::new() {
            ctx.set_clipboard_backend(clipboard);
        } else {
            log::error!("Failed to initialize clipboard backend");
        }

        let font_size = 14.0;
        ctx.fonts().add_font(&[FontSource::TtfData {
            data: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/res/Roboto-Regular.ttf"
            )),
            size_pixels: font_size,
            config: Some(FontConfig {
                rasterizer_multiply: 1.5,
                oversample_h: 4,
                oversample_v: 4,
                ..Default::default()
            }),
        }]);

        let renderer_config = RendererConfig {
            texture_format: swapchain_format,
            ..Default::default()
        };
        let renderer = ImguiRenderer::new(&mut ctx, device, queue, renderer_config);
        (Self(ctx, renderer), UiWinitPlatform(platform))
    }

    pub fn new_headless(
        size: (u32, u32),
        device: &Device,
        queue: &Queue,
        swapchain_format: TextureFormat,
    ) -> (Self, ()) {
        let mut ctx = Context::create();
        ctx.set_ini_filename(None);

        if let Some(clipboard) = ClipboardSupport::new() {
            ctx.set_clipboard_backend(clipboard);
        } else {
            log::error!("Failed to initialize clipboard backend");
        }

        let font_size = 14.0;
        ctx.fonts().add_font(&[FontSource::TtfData {
            data: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/res/Roboto-Regular.ttf"
            )),
            size_pixels: font_size,
            config: Some(FontConfig {
                rasterizer_multiply: 1.5,
                oversample_h: 4,
                oversample_v: 4,
                ..Default::default()
            }),
        }]);

        ctx.io_mut().display_size = [size.0 as f32, size.1 as f32];

        let renderer_config = RendererConfig {
            texture_format: swapchain_format,
            ..Default::default()
        };
        let renderer = ImguiRenderer::new(&mut ctx, device, queue, renderer_config);
        (Self(ctx, renderer), ())
    }

    pub fn ui_frame(&mut self) -> UiFrame {
        UiFrame(self.0.frame())
    }
}

#[derive(Debug)]
pub struct UiFrame<'a>(&'a mut imgui::Ui);
