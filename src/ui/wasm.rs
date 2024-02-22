use std::convert::Infallible;
use web_sys::{self, Document, HtmlElement};
use winit::window::Window;

pub type UiDrawError = Infallible;

pub struct UiWinitPlatform;

pub trait UiPlatform {
    fn prepare_render(&mut self, _frame: UiFrame, _window: &Window) {}
}

impl UiPlatform for UiWinitPlatform {}
impl UiPlatform for () {}

#[derive(Debug)]
pub struct Ui {
    pub(crate) document: Document,
    pub(crate) body: HtmlElement,
    pub(crate) input_forms_created: bool,
}

pub type UiContext<'a> = &'a mut Ui;
pub type UiFrame<'a> = &'a mut Ui;

impl Ui {
    pub fn new_frame(&mut self) -> UiFrame<'_> {
        self
    }

    pub fn context_mut(&mut self) -> UiContext<'_> {
        self
    }

    pub fn new(
        _window: &Window,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _swapchain_format: wgpu::TextureFormat,
    ) -> (Self, UiWinitPlatform) {
        log::debug!("aftgraphs::ui::new: Creating ui");

        // All of these unwraps are checked in sim_main before this is run
        let html_window = unsafe { web_sys::window().unwrap_unchecked() };
        let document = unsafe { html_window.document().unwrap_unchecked() };
        let body = unsafe { document.body().unwrap_unchecked() };

        (
            Self {
                body,
                document,
                input_forms_created: false,
            },
            UiWinitPlatform,
        )
    }

    pub fn new_headless() -> (Self, ()) {
        unreachable!("no headless available in wasm")
    }

    pub fn draw(
        &mut self,
        _render_pass: &mut wgpu::RenderPass<'_>,
        _queue: &wgpu::Queue,
        _device: &wgpu::Device,
    ) -> Result<(), UiDrawError> {
        Ok(())
    }
}
