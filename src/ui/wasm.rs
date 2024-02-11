use wgpu::{Device, Queue, TextureFormat};
use winit::window::Window;

pub type UiPlatform = ();

#[derive(Debug)]
pub struct Ui;

impl Ui {
    pub fn new(
        _window: &Window,
        _device: &Device,
        _queue: &Queue,
        _swapchain_format: TextureFormat,
    ) -> (Self, UiPlatform) {
        todo!("Implement UI for wasm")
    }

    pub fn new_headless() -> (Self, UiPlatform) {
        unreachable!("no headless available in wasm")
    }
}

#[derive(Debug)]
pub struct UiFrame;
