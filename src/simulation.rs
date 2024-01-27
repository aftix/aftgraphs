use async_mutex::Mutex;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::block_on;
use crate::render::Renderer;

pub trait Simulation {}

pub struct SimulationContext {
    event_loop: EventLoop<()>,
    renderer: Arc<Mutex<Renderer>>,
    window: Option<Window>,
}

mod builder;
pub use builder::{BuilderState, SimulationBuilder};

impl SimulationContext {
    pub async fn run(self) {
        self.event_loop
            .run(move |event, win_target| match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let renderer = self.renderer.clone();
                    block_on(async move {
                        let mut renderer = renderer.lock().await;
                        renderer.config.as_mut().unwrap().width = size.width;
                        renderer.config.as_mut().unwrap().height = size.height;
                        renderer
                            .surface
                            .as_ref()
                            .unwrap()
                            .configure(&renderer.device, renderer.config.as_ref().unwrap());
                    });
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let renderer = self.renderer.clone();
                    block_on(async move {
                        let renderer = renderer.lock().await;
                        renderer.render(0..3, 0..1, None).await;
                    });
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    win_target.exit();
                }
                _ => (),
            })
            .unwrap();
    }
}
