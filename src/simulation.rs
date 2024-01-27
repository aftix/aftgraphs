use async_mutex::Mutex;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::block_on;
use crate::render::Renderer;

pub trait Simulation: 'static {
    #[allow(async_fn_in_trait)]
    async fn render(&mut self, renderer: Arc<Mutex<Renderer>>, out_img: Arc<Mutex<Vec<u8>>>);
}

pub struct SimulationContext<T: Simulation> {
    event_loop: EventLoop<()>,
    renderer: Arc<Mutex<Renderer>>,
    window: Option<Window>,
    simulation: T,
}

mod builder;
pub use builder::{BuilderState, SimulationBuilder};

impl<T: Simulation> SimulationContext<T> {
    pub async fn run(self, out_img: Arc<Mutex<Vec<u8>>>) {
        let simulation = Arc::new(Mutex::new(self.simulation));
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
                    let simulation = simulation.clone();
                    let out_img = out_img.clone();
                    block_on(async move {
                        simulation.lock().await.render(renderer, out_img).await;
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
